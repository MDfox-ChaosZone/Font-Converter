"""Exercise the GPUI controls and validate real downloaded files on localhost."""

import argparse
import gzip
import hashlib
import io
import json
import os
import statistics
import tempfile
import time
from pathlib import Path

from fontTools.ttLib import TTFont
from PIL import Image
from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
URL = os.environ.get("FONT_WEB_URL", "http://127.0.0.1:8137").rstrip("/")
BLUE = (36, 94, 166)


def show(page):
    start = time.perf_counter()
    page.goto(f"{URL}/gpui/", wait_until="commit")
    while time.perf_counter() - start < 30:
        image = Image.open(io.BytesIO(page.screenshot())).convert("RGB")
        if any(
            color == BLUE and count > 300
            for count, color in image.getcolors(image.width * image.height)
        ):
            return round((time.perf_counter() - start) * 1000)
    raise RuntimeError(
        f"gpui did not paint the choose button: {page.locator('body').inner_text()}"
    )


def click(page, label):
    # GPUI owns one canvas. Locate the enabled button by its painted
    # background; the second one only becomes enabled after conversion.
    image = Image.open(io.BytesIO(page.screenshot())).convert("RGB")
    for y in range(image.height):
        runs = []
        for x in range(image.width):
            if image.getpixel((x, y)) == BLUE:
                if not runs or runs[-1][-1] != x - 1:
                    runs.append([x])
                else:
                    runs[-1].append(x)
        buttons = [r for r in runs if len(r) > 45]
        index = int(label == "Download")
        if len(buttons) > index:
            run = buttons[index]
            page.mouse.click(run[len(run) // 2], y + 3)
            return
    raise AssertionError(f"{label} is not painted as enabled")


def select(page, path):
    with page.expect_file_chooser(timeout=6000) as opened:
        click(page, "Choose font")
    opened.value.set_files(path)
    page.wait_for_function("!fontConverter.pending()")
    page.wait_for_timeout(130)  # let the Rust view apply the worker result


def convert(page, path, directory):
    select(page, path)
    assert page.evaluate("fontConverter.ready()"), page.evaluate(
        "fontConverter.status()"
    )
    with page.expect_download(timeout=6000) as event:
        click(page, "Download")
    result = directory / event.value.suggested_filename
    event.value.save_as(result)
    return result


def verify(browser, directory):
    page = browser.new_page(viewport={"width": 900, "height": 520})
    errors = []
    page.on("pageerror", lambda e: errors.append(str(e)))
    requests = []
    page.on("request", lambda r: requests.append((r.method, r.url)))
    first = show(page)
    assert not any(url.endswith("/codec.wasm") for _, url in requests)
    bad = directory / "invalid.ttf"
    bad.write_bytes(b"not a font")
    select(page, bad)
    assert not page.evaluate("fontConverter.ready()")
    assert "Choose a TTF" in page.evaluate("fontConverter.status()")
    results = {}
    for name in ("Abel-Regular.ttf", "SourceCodePro-Regular.otf"):
        source = ROOT / "tests/fixtures" / name
        encoded = convert(page, source, directory)
        assert encoded.suffix == ".woff2" and encoded.read_bytes()[:4] == b"wOF2"
        decoded = convert(page, encoded, directory)
        assert decoded.name == name
        with TTFont(source) as a, TTFont(decoded) as b:
            assert a.getBestCmap() == b.getBestCmap()
            assert a.getGlyphOrder() == b.getGlyphOrder()
            assert a["hmtx"].metrics == b["hmtx"].metrics
        results[name] = [
            len(encoded.read_bytes()),
            hashlib.sha256(encoded.read_bytes()).hexdigest(),
            hashlib.sha256(decoded.read_bytes()).hexdigest(),
        ]
    assert not errors, errors
    assert all(
        method == "GET" and url.startswith(URL + "/") for method, url in requests
    ), requests
    page.context.close()
    return {"first_visible_ms": first, "outputs": results}


def measure(browser):
    dist = ROOT / "web/dist"
    measurements = []
    for _ in range(3):
        with browser.new_context(viewport={"width": 900, "height": 520}) as context:
            page = context.new_page()
            elapsed = show(page)
            resources = page.evaluate("""() => [
                performance.getEntriesByType('navigation')[0].name,
                ...performance.getEntriesByType('resource').map(r => r.name)
            ]""")
            files = set()
            for url in resources:
                if url.startswith(URL + "/") and "favicon" not in url:
                    path = dist / url[len(URL) + 1 :]
                    files.add(path / "index.html" if path.is_dir() else path)
            sizes = [f.read_bytes() for f in sorted(files)]
            measurements.append(elapsed)
            print(
                elapsed,
                "ms",
                sum(map(len, sizes)),
                "bytes",
                sum(len(gzip.compress(b, compresslevel=9, mtime=0)) for b in sizes),
                "gzip bytes",
                flush=True,
            )
    print(
        "Median visible time (ms)",
        statistics.median(measurements),
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--measure", action="store_true")
    parser.add_argument(
        "--browser", choices=["chromium", "firefox"], default="chromium"
    )
    args = parser.parse_args()
    with sync_playwright() as pw, tempfile.TemporaryDirectory() as tmp:
        if args.browser == "firefox":
            browser = pw.firefox.launch()
        else:
            browser = pw.chromium.launch(
                executable_path=os.environ.get("CHROMIUM_PATH"),
                args=["--enable-unsafe-swiftshader"],
            )
        print(json.dumps(verify(browser, Path(tmp))), flush=True)
        if args.measure:
            measure(browser)
        print(args.browser, browser.version)
        browser.close()
