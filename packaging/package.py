#!/usr/bin/env python3
"""Native release packaging; Python 3.11+, no application runtime dependency."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parent.parent
IDENTIFIER = 'io.github.mdfox-chaoszone.fontconverter'

def run(*args, **kwargs):
    subprocess.run([str(a) for a in args], check=True, **kwargs)

def archive_zip(path, source):
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as archive:
        for file in sorted(source.rglob('*')):
            if file.is_file():
                archive.write(file, file.relative_to(source))

def notices(stage):
    for name in ('LICENSE', 'THIRD_PARTY_LICENSES'):
        shutil.copy2(ROOT / name, stage / name)
    shutil.copytree(ROOT / 'licenses', stage / 'licenses', dirs_exist_ok=True)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', required=True)
    parser.add_argument('--binaries-dir', type=Path)
    parser.add_argument('--skip-build', action='store_true')
    args = parser.parse_args()
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['workspace']['package']['version']
    target = args.target
    arch = 'arm64' if target.startswith('aarch64') else 'x64'
    os_name = 'windows' if 'windows' in target else 'macos' if 'apple' in target else 'linux'
    expected = {'Windows': 'windows', 'Darwin': 'macos', 'Linux': 'linux'}[platform.system()]
    if os_name != expected:
        parser.error('Package on the target OS; cross-platform packaging is not supported')
    if not args.skip_build:
        run('cargo', 'build', '--release', '--locked', '--target', target, cwd=ROOT)
    binaries = args.binaries_dir or ROOT / 'target' / target / 'release'
    dist = ROOT / 'dist' / target
    dist.mkdir(parents=True, exist_ok=True)
    suffix = '.exe' if os_name == 'windows' else ''
    gui = binaries / ('font-converter' + suffix)
    cli = binaries / ('font-converter-cli' + suffix)
    if not gui.is_file() or not cli.is_file():
        parser.error(f'Missing release binaries in {binaries}')
    base = f'font-converter-{version}-{os_name}-{arch}'
    with tempfile.TemporaryDirectory(prefix='font-converter-package-') as tmp:
        stage = Path(tmp)
        cli_stage = stage / 'cli'
        cli_stage.mkdir()
        shutil.copy2(cli, cli_stage / cli.name)
        notices(cli_stage)
        if os_name == 'windows':
            archive_zip(dist / f'{base}-cli.zip', cli_stage)
            app = stage / 'portable'
            app.mkdir()
            shutil.copy2(gui, app / gui.name)
            notices(app)
            archive_zip(dist / f'{base}.zip', app)
            shutil.copy2(gui, dist / gui.name)
        else:
            with tarfile.open(dist / f'{base}-cli.tar.gz', 'w:gz') as archive:
                archive.add(cli_stage, arcname='font-converter-cli')
            if os_name == 'macos':
                app = stage / 'Font Converter.app'
                executable = app / 'Contents/MacOS'
                resources = app / 'Contents/Resources'
                executable.mkdir(parents=True)
                resources.mkdir()
                shutil.copy2(gui, executable / gui.name)
                shutil.copy2(ROOT / 'assets/icons/icon.icns', resources / 'icon.icns')
                notices(resources)
                import plistlib
                (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
                    'CFBundleName': 'Font Converter', 'CFBundleDisplayName': 'Font Converter',
                    'CFBundleIdentifier': IDENTIFIER, 'CFBundleExecutable': 'font-converter',
                    'CFBundlePackageType': 'APPL', 'CFBundleShortVersionString': version,
                    'CFBundleVersion': version, 'CFBundleIconFile': 'icon.icns',
                    'NSHighResolutionCapable': True, 'LSMinimumSystemVersion': '15.0',
                }))
                identity = os.environ.get('APPLE_SIGNING_IDENTITY')
                run('codesign', '--force', '--sign', identity or '-', *(['--timestamp', '--options', 'runtime'] if identity else []), app)
                run('codesign', '--verify', '--deep', '--strict', app)
                run('ditto', '-c', '-k', '--keepParent', app, dist / f'{base}-app.zip')
                dmg = dist / f'{base}.dmg'
                run('hdiutil', 'create', '-volname', 'Font Converter', '-srcfolder', app, '-ov', '-format', 'UDZO', dmg)
                if identity:
                    run('codesign', '--sign', identity, '--timestamp', dmg)
                profile = os.environ.get('APPLE_NOTARY_PROFILE')
                if profile:
                    result = subprocess.run(['xcrun','notarytool','submit',str(dmg),'--keychain-profile',profile,'--wait','--output-format','json'],check=True,capture_output=True,text=True)
                    if json.loads(result.stdout).get('status') != 'Accepted':
                        raise RuntimeError('Apple notarization did not return Accepted')
                    run('xcrun', 'stapler', 'staple', dmg)
                    run('xcrun', 'stapler', 'validate', dmg)
            else:
                linux_packages(stage, dist, gui, base, version, target)
    outputs = [f for f in dist.iterdir() if f.is_file() and f.name != 'SHA256SUMS' and f.name != 'build.json']
    (dist / 'SHA256SUMS').write_text(''.join(f'{hashlib.sha256(f.read_bytes()).hexdigest()}  {f.name}\n' for f in sorted(outputs)), encoding='utf-8')
    revision = subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    (dist / 'build.json').write_text(json.dumps({'version': version,'target': target,'revision': revision,'workingTreeDirty': bool(subprocess.check_output(['git','status','--porcelain'],cwd=ROOT))},indent=2),encoding='utf-8')
    print(dist)

def linux_packages(stage, dist, gui, base, version, target):
    root = stage / 'linux'
    binary_dir = root / 'usr/bin'
    desktop_dir = root / 'usr/share/applications'
    icon_dir = root / 'usr/share/icons/hicolor/256x256/apps'
    doc = root / 'usr/share/doc/font-converter'
    for directory in (binary_dir, desktop_dir, icon_dir, doc):
        directory.mkdir(parents=True)
    shutil.copy2(gui,binary_dir / 'font-converter')
    shutil.copy2(ROOT / 'assets/icons/icon.png',icon_dir / 'font-converter.png')
    notices(doc)
    desktop = '[Desktop Entry]\nType=Application\nName=Font Converter\nExec=font-converter\nIcon=font-converter\nCategories=Utility;\nTerminal=false\n'
    (desktop_dir / f'{IDENTIFIER}.desktop').write_text(desktop)
    deb_root = stage / 'deb'
    shutil.copytree(root,deb_root)
    (deb_root / 'DEBIAN').mkdir()
    deb_arch = 'arm64' if target.startswith('aarch64') else 'amd64'
    (deb_root / 'DEBIAN/control').write_text(f'Package: font-converter\nVersion: {version}\nArchitecture: {deb_arch}\nMaintainer: Font Converter contributors\nSection: utils\nPriority: optional\nDepends: libc6, libstdc++6, libfontconfig1, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libx11-6, libxcb1, libvulkan1\nDescription: Native TTF/OTF and WOFF2 font converter\n')
    run('dpkg-deb','--root-owner-group','--build',deb_root,dist / f'{base}.deb')
    rpm = stage / 'rpm'
    for name in ('BUILD','BUILDROOT','RPMS','SOURCES','SPECS','SRPMS'):
        (rpm/name).mkdir(parents=True)
    with tarfile.open(rpm/'SOURCES/app.tar.gz','w:gz') as archive:
        archive.add(root/'usr',arcname='usr')
    spec = rpm/'SPECS/font-converter.spec'
    spec.write_text(f'''Name: font-converter
Version: {version}
Release: 1
Summary: Native font format converter
License: MIT AND Apache-2.0
Source0: app.tar.gz
AutoReqProv: yes
Requires: vulkan-loader, fontconfig, libxkbcommon, libxkbcommon-x11, wayland-libs
%description
Convert TTF/OTF fonts and WOFF2 fonts locally.
%prep
%build
%install
mkdir -p %{{buildroot}}
tar -xzf %{{SOURCE0}} -C %{{buildroot}}
%files
/usr/bin/font-converter
/usr/share/applications/*
/usr/share/icons/hicolor/256x256/apps/*
/usr/share/doc/font-converter
''')
    run('rpmbuild','-bb','--define',f'_topdir {rpm}','--define','_build_id_links none',spec)
    for file in (rpm/'RPMS').rglob('*.rpm'):
        shutil.copy2(file,dist/f'{base}.rpm')
    appdir = stage/'AppDir'
    shutil.copytree(root,appdir)
    (appdir/f'{IDENTIFIER}.desktop').write_text(desktop)
    shutil.copy2(ROOT/'assets/icons/icon.png',appdir/'font-converter.png')
    launcher = appdir/'AppRun'
    launcher.write_text('#!/bin/sh\nAPPDIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nexec "$APPDIR/usr/bin/font-converter" "$@"\n')
    launcher.chmod(0o755)
    tool = os.environ.get('APPIMAGETOOL')
    if not tool:
        raise RuntimeError('Set APPIMAGETOOL to appimagetool 1.9.1 executable')
    run(tool,'--appimage-extract-and-run',appdir,dist/f'{base}.AppImage',env={**os.environ,'ARCH': 'aarch64' if target.startswith('aarch64') else 'x86_64'})

if __name__ == '__main__':
    main()
