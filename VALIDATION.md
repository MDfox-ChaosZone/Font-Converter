# Validation

Current state of verification for Font Converter 2.0.0. Automated checks were rerun on 2026-11-14 on Windows x64 with Rust 1.100.0 (MSVC) and GPUI 0.3.12. The size, memory and native runtime observations below are from the earlier Rust 1.99.1 / GPUI 0.3.10 release build. Linux x64 and macOS Apple Silicon package and runtime checks are recorded below.

## Automated checks (Windows x64)

- `cargo test --locked --workspace`: 19 tests pass (7 desktop/settings, 3 CLI integration, 9 engine integration). A directory-symlink traversal test runs only on Unix.
- `cargo fmt --all -- --check` and `cargo clippy --locked --workspace --all-targets -- -D warnings`: pass.
- Engine: the pinned Abel TTF and Source Code Pro OTF fixtures round-trip through WOFF2 with the correct TTF/OTF output choice. Glyph advances, bounds and outline commands, character mappings, units per em, glyph counts and name tables all match. This checks font semantics on these fixtures; it does not check byte equality or compatibility with every font.
- Engine edge cases: output collisions, outputs replacing inputs, existing-output policies, Unicode and nested paths, duplicate inputs, missing, malformed and oversized inputs, blocked output directories, atomic cleanup on failure, and cancellation.
- Desktop (headless GPUI): 10,000 rows render lazily and keyboard selection works (Down, End). Also covered: stale batch events are ignored, controls lock during conversion, a failed worker releases the UI, removing rows keeps ids aligned, platform file drops dispatch, and settings recover from corrupt or old formats. These are headless tests, not real GPU or OS drag-and-drop runs.

## Size and memory

The desktop now depends on `gpui-pre` / `gpui-pre-platform` 0.3.12 directly, with no component library. The measurements below predate this dependency update; no new release size, startup or memory benchmark has been run. Release profile: `opt-level = "s"`, fat LTO, one codegen unit, stripped symbols. The engine, including its C++ WOFF2/Brotli, stays at `opt-level = 3`.

| Build | `font-converter.exe` | Working set | Private bytes |
| --- | --- | --- | --- |
| With GPUI Kit 0.7.3, default profile | 21.0 MB | 78.0 MB | 93.3 MB |
| Previous GPUI 0.3.10 build | 9.0 MB | 74.1 MB | 93.8 MB |

Memory is the average of three cold starts, sampled after 5 s on the empty window. It is dominated by the GPU driver and renderer. `font-converter-cli.exe` is 1.6 MB.

## Native runtime checks (previous Windows x64 release build)

- Font paths passed on the command line are queued at startup.
- Convert turned 4 TTF and 1 OTF copies into WOFF2, with size changes of −67% and −42%.
- The ⋯ menu opens under its button and closes on an outside click.
- Hovering a row shows its reveal and remove buttons.
- Switching light/dark and English/Chinese updates the window immediately.
- The screenshots in `assets/screenshots` come from this build.
- CLI: with 10,000 real font inputs, a dry-run scan wrote no output. A console-control interrupt during conversion exited with **130**, `interrupted=true`, and a complete JSON report.

Still unverified on Windows: native OS drag-and-drop onto the window, a real-mouse scrollbar drag, high-DPI changes, screen-reader output, keyboard focus order inside the ⋯ menu, and the close-while-converting prompt.

## Packaged GPUI application (2026-11-15)

GitHub-hosted Ubuntu 24.04 x64 and macOS 15 Apple Silicon runners built the same pure GPUI 0.3.12 release snapshot (`d240ede63ea3862f3b3cd3c58e532ac584bbe78d`). All 20 workspace tests passed on each OS. SHA-256 checks passed both on the runners and on the downloaded artifacts. [Build and package run](https://github.com/MDfox-ChaosZone/Font-Converter/actions/runs/43695088914)

| Check | Ubuntu x64 (X11) | macOS Apple Silicon |
|---|---|---|
| Packages | DEB, RPM, AppImage and CLI archive produced. DEB installed and launched. RPM metadata and AppImage extraction checked. | DMG verified and mounted. The app copied from the volume launched. Both it and the app extracted from ZIP passed strict ad hoc signature verification. |
| Native file drop | A GTK source delivered six file URLs over X11 to the empty GPUI window. | An AppKit dragging session delivered six file URLs to the empty GPUI window and reported a copy operation. |
| Mixed batch | All six dropped TTF/OTF inputs converted. A separate launch with a fixture folder also converted all six. | All six dropped TTF/OTF inputs converted. A separate launch with a fixture folder also converted all six. |
| GUI cancellation | Clicked Cancel after output began. 8 of 512 files completed. | Clicked Cancel after output began. 6 of 512 files completed. |
| Packaged CLI | Six mixed inputs converted. SIGINT exited 130 with 6 completed and 506 cancelled. | Six mixed inputs converted. SIGINT exited 130 with 6 completed and 506 cancelled. |

The native GUI tests used real packaged executables and OS pointer events on hosted desktops. They verified the WOFF2 magic and declared byte length of every output in the six-file batch. After cancellation the process stayed alive and output counts stayed constant during a further four-second observation. Counts depend on timing. The first X11 test driver used screen-relative coordinates affected by the window frame. Switching to client-relative coordinates passed with the same binary. [Packaged interaction logs](https://github.com/MDfox-ChaosZone/Font-Converter/actions/runs/43695789647)

The extracted AppImage also passed under native Wayland on Ubuntu x64. With Xwayland disabled in a nested Sway session, a separate GTK application dragged six file URLs to the GPUI window. All six converted to WOFF2, as did the separate six-file folder batch. Clicking Cancel after output began left 8 of 512 outputs, with the count stable for four more seconds and the app still running. [Wayland interaction logs](https://github.com/MDfox-ChaosZone/Font-Converter/actions/runs/43695883557)

## Platform and release boundaries

CI and the draft-release workflow cover Windows x64/ARM64, Linux x64/ARM64 and macOS Apple Silicon. The November 15 validation covers Ubuntu x64 and macOS Apple Silicon on hosted runners. Linux ARM64, Windows ARM64, RPM installation on a Fedora-family system and macOS Gatekeeper behavior for a quarantined public download remain unverified for this snapshot. Linux packages rely on system libraries and GPU drivers. macOS uses ad hoc signing unless signing and notarization credentials are configured. Windows packages are unsigned.
