"""Build the static GPUI client from the same checkout. See README.md."""

import argparse
import os
import shutil
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

WEB = Path(__file__).resolve().parent
ROOT = WEB.parent
DIST = WEB / "dist"
TARGET = WEB / "target"
WASM = "wasm32-unknown-unknown"


def build_codec(sdk):
    woff = ROOT / "vendor/woff2"
    brotli = ROOT / "vendor/brotli/c"
    obj = TARGET / "codec"
    obj.mkdir(parents=True, exist_ok=True)
    dest = DIST / "shared"
    dest.mkdir(parents=True, exist_ok=True)
    sources = [ROOT / "engine/native/codec.cc"]
    sources.extend(
        woff / "src" / (name + ".cc")
        for name in (
            "table_tags",
            "variable_length",
            "woff2_common",
            "font",
            "glyph",
            "normalize",
            "transform",
            "woff2_enc",
            "woff2_dec",
            "woff2_out",
        )
    )
    for part in ("enc", "dec", "common"):
        sources.extend(sorted((brotli / part).glob("*.c")))
    includes = [
        "-I" + str(d) for d in (woff / "include", woff / "src", brotli / "include")
    ]

    def compile_one(source):
        out = obj / ("-".join(source.relative_to(ROOT).parts) + ".o")
        cpp = source.suffix == ".cc"
        compiler = sdk / "bin" / ("clang++" if cpp else "clang")
        args = ["-std=c++11", "-fno-exceptions"] if cpp else []
        if source.name == "codec.cc":
            args[-1] = (
                "-fexceptions"  # Parse the shared native boundary with its catch clauses.
            )
        subprocess.run(
            [compiler, "-O3", *args, *includes, "-c", source, "-o", out], check=True
        )
        return out

    with ThreadPoolExecutor(max_workers=min(os.cpu_count() or 1, 8)) as pool:
        objects = list(pool.map(compile_one, sources))
    subprocess.run(
        [
            sdk / "bin/clang++",
            "-mexec-model=reactor",
            "-Wl,--strip-all",
            "-Wl,--export=malloc,--export=free,--export=fc_capacity,--export=fc_convert",
            "-Wl,--max-memory=536870912",
            *objects,
            "-o",
            dest / "codec.wasm",
        ],
        check=True,
    )


def build_ui(sdk, bindgen):
    env = os.environ.copy()
    env.update(
        CC_wasm32_unknown_unknown=str(sdk / "bin/clang"),
        AR_wasm32_unknown_unknown=str(sdk / "bin/llvm-ar"),
        CFLAGS_wasm32_unknown_unknown="--target=wasm32-unknown-unknown",
    )
    subprocess.run(
        ["cargo", "build", "--locked", "--release", "--target", WASM],
        cwd=WEB,
        env=env,
        check=True,
    )
    release = TARGET / WASM / "release"
    subprocess.run(
        [
            bindgen,
            release / "font_web_gpui.wasm",
            "--target",
            "web",
            "--no-typescript",
            "--out-dir",
            DIST / "gpui",
            "--out-name",
            "gpui",
        ],
        check=True,
    )
    subprocess.run([sdk / "bin/llvm-strip", "--strip-all", DIST / "gpui/gpui_bg.wasm"], check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--sdk", type=Path, required=True, help="WASI SDK directory")
    parser.add_argument(
        "--bindgen", default="wasm-bindgen", help="wasm-bindgen CLI executable"
    )
    parser.add_argument("--codec-only", action="store_true")
    args = parser.parse_args()
    sdk = args.sdk.resolve()
    build_codec(sdk)
    if not args.codec_only:
        for name in ("gpui", "shared"):
            (DIST / name).mkdir(parents=True, exist_ok=True)
        build_ui(sdk, args.bindgen)
        for name in ("gpui", "shared"):
            for source in (WEB / name).glob("*"):
                if source.suffix in (".html", ".js", ".css"):
                    shutil.copy2(source, DIST / name)
        shutil.copy2(WEB / "index.html", DIST / "index.html")
        for package in ("woff2", "brotli"):
            shutil.copy2(
                ROOT / "vendor" / package / "LICENSE",
                DIST / "shared" / (package + "-LICENSE.txt"),
            )
        shutil.copy2(ROOT / "tests/fixtures/SourceCodePro-LICENSE.txt", DIST / "gpui")

        shutil.copy2(ROOT / "LICENSE", DIST / "LICENSE.txt")
        shutil.copy2(ROOT / "licenses/GPUI-APACHE-2.0.txt", DIST / "Apache-2.0.txt")
