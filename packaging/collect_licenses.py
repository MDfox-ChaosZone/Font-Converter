#!/usr/bin/env python3
"""Collect Cargo package declarations and published license/notice texts."""
import hashlib
import json
from pathlib import Path
import subprocess
root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(['cargo','metadata','--locked','--offline','--format-version','1'],cwd=root))
texts = {}
entries = []
missing = []
for package in sorted(metadata['packages'],key=lambda p:(p['name'],p['version'])):
    if package['source'] is None:
        continue
    source = Path(package['manifest_path']).parent
    candidates = [path for path in source.iterdir() if path.is_file() and path.name.upper().startswith(('LICENSE','LICENCE','COPYING','NOTICE','UNLICENSE'))]
    if package.get('license_file'):
        specified = source / package['license_file']
        if specified.is_file() and specified not in candidates:
            candidates.append(specified)
    references = []
    for path in sorted(candidates):
        text = path.read_text(encoding='utf-8',errors='replace')
        digest = hashlib.sha256(text.encode()).hexdigest()
        texts[digest] = text
        references.append(f'{path.name}: {digest}')
    if not references:
        missing.append(f"{package['name']} {package['version']}")
    entries.append(f"{package['name']} {package['version']}\nLicense: {package.get('license') or 'See source license file'}\nSource: {package.get('repository') or package['source']}\n" + '\n'.join(references))
output = root/'licenses/Cargo-dependencies.txt'
output.write_text('Cargo dependency declarations and published license texts\nGenerated from Cargo.lock; includes platform and optional packages.\n\n' + '\n\n'.join(entries) + '\n\n' + '\n\n'.join(f'License text SHA256: {digest}\n{text}' for digest,text in sorted(texts.items())) + '\n\nPackages whose published source has no top-level license text (SPDX declaration above):\n'+'\n'.join(missing)+'\n',encoding='utf-8')
print(f'{len(entries)} packages, {len(texts)} unique texts, {len(missing)} declaration-only packages')
