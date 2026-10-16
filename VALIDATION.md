# Validation

Current state of verification for Font Converter 2.0.0. Local checks ran on Windows x64 with Rust 1.97.1 (MSVC). Other platforms rely on CI, which has not yet run remotely.

## Automated checks (Windows x64)

- `cargo test --locked --workspace`: 19 tests pass (7 desktop/settings, 3 CLI integration, 9 engine integration). A directory-symlink traversal test runs only on Unix.
- `cargo fmt --all -- --check` and `cargo clippy --locked --workspace --all-targets -- -D warnings`: pass.
- Engine: the pinned Abel TTF and Source Code Pro OTF fixtures round-trip through WOFF2 with the correct TTF/OTF output choice. Glyph advances, bounds and outline commands, character mappings, units per em, glyph counts and name tables all match. This checks font semantics on these fixtures; it does not check byte equality or compatibility with every font.
- Engine edge cases: output collisions, outputs replacing inputs, existing-output policies, Unicode and nested paths, duplicate inputs, missing, malformed and oversized inputs, blocked output directories, atomic cleanup on failure, and cancellation.
- Desktop (headless GPUI): 10,000 rows render lazily and keyboard selection works (Down, End). Also covered: stale batch events are ignored, controls lock during conversion, a failed worker releases the UI, removing rows keeps ids aligned, platform file drops dispatch, and settings recover from corrupt or old formats. These are headless tests, not real GPU or OS drag-and-drop runs.

## Size and memory

The desktop depends on `gpui-pre` / `gpui-pre-platform` 0.3.9 directly, with no component library. Release profile: `opt-level = "s"`, fat LTO, one codegen unit, stripped symbols. The engine, including its C++ WOFF2/Brotli, stays at `opt-level = 3`.

| Build | `font-converter.exe` | Working set | Private bytes |
| --- | --- | --- | --- |
| With GPUI Kit 0.7.3, default profile | 21.0 MB | 78.0 MB | 93.3 MB |
| Current | 9.0 MB | 74.1 MB | 93.8 MB |

Memory is the average of three cold starts, sampled after 5 s on the empty window. It is dominated by the GPU driver and renderer. `font-converter-cli.exe` is 1.6 MB.

## Native runtime checks (Windows x64 release build)

- Font paths passed on the command line are queued at startup.
- Convert turned 4 TTF and 1 OTF copies into WOFF2, with size changes of −67% and −42%.
- The ⋯ menu opens under its button and closes on an outside click.
- Hovering a row shows its reveal and remove buttons.
- Switching light/dark and English/Chinese updates the window immediately.
- The screenshots in `assets/screenshots` come from this build.
- CLI: with 10,000 real font inputs, a dry-run scan wrote no output. A console-control interrupt during conversion exited with **130**, `interrupted=true`, and a complete JSON report.

Not verified: native OS drag-and-drop onto the window, a real-mouse scrollbar drag, high-DPI changes, screen-reader output, keyboard focus order inside the ⋯ menu, and the close-while-converting prompt.

## Platform and release boundaries

CI and the draft-release workflow cover Windows x64/ARM64, Linux x64/ARM64 and macOS Apple Silicon, but none of it has run remotely yet. Linux, macOS and Windows ARM64 builds, packages and runtime are unverified. Linux packages rely on system libraries and GPU drivers. macOS uses ad hoc signing unless signing and notarization credentials are configured. Windows packages are unsigned.
