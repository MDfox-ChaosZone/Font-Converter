# Font Converter

[English](#english) · [简体中文](#简体中文)

## 简体中文

`Font Converter` 是基于 Tauri 2 和 Leptos 的跨平台字体转换器，同时提供桌面 GUI 和跨平台 CLI。
转换核心使用 Google 官方 [`google/woff2`](https://github.com/google/woff2) 参考实现及其固定版本的 Brotli 子模块。

### 功能

- TTF、OTF 与 WOFF2 之间自动判断方向并转换。
- 支持文件和目录输入，目录会递归扫描。
- 默认将结果写入源文件旁，也可以指定统一的输出目录。
- 不覆盖已有输出文件；CLI 支持人类可读输出和 JSON 报告。
- GUI 后台并行转换，最多同时处理 4 个字体。
- Windows x64、Linux x64、macOS Intel 和 Apple Silicon 构建。

WOFF2 解码时会读取文件头中的 SFNT flavor：TrueType 输出 `.ttf`，CFF/OpenType 输出 `.otf`。
暂不支持 WOFF1 和字体集合。

### CLI

构建并运行：

```bash
cargo run -p font-converter-cli -- ./fonts
cargo run -p font-converter-cli -- -o ./converted ./font.ttf ./font.otf
```

安装后的用法：

```text
font-converter-cli [OPTIONS] <PATH>...

Options:
  -o, --output-dir <DIR>  Put all generated files in this existing directory
      --json              Print one JSON report instead of progress output
  -q, --quiet             Suppress human-readable progress output
  -h, --help              Print help
  -V, --version           Print version
```

CLI 使用退出码区分成功（`0`）、转换失败（`1`）、参数或没有可转换输入（`2`）以及 Ctrl+C（`130`）。

### 开发环境

通用依赖：

```text
Rust stable（包含 wasm32-unknown-unknown target）
Trunk
Tauri CLI 2
```

安装工具：

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
cargo install --locked tauri-cli
git submodule update --init --recursive
```

各平台还需要：

- Windows：Microsoft C++ Build Tools 和 WebView2。
- macOS：Xcode Command Line Tools。
- Debian/Ubuntu Linux：`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`。

启动 GUI：

```bash
cargo tauri dev
```

检查和测试：

```bash
cargo fmt --all -- --check
cargo test -p font-converter-shared -p font-converter-core -p font-converter-cli -p font-converter
cargo clippy -p font-converter-shared -p font-converter-core -p font-converter-cli -p font-converter --all-targets -- -D warnings
cargo check -p font-converter-frontend --target wasm32-unknown-unknown
trunk build --config frontend/Trunk.toml --release
```

核心转换代码位于 `core`，GUI 和 CLI 均依赖它。Google WOFF2 的 C ABI 只在
`core/native/woff2_wrapper.cc` 和 `core/src/converter.rs` 中适配。

真实字体测试可通过 `FONT_CONVERTER_TEST_FONT=/path/to/font.ttf` 和
`FONT_CONVERTER_TEST_OTF=/path/to/font.otf` 启用。

### 发布

推送 `v*` 标签会创建草稿 GitHub Release，并构建：

- Windows：只有 `Font-Converter-windows-x64-portable.exe`，不生成 MSI/NSIS 安装包；Windows 10 1803+ 和 Windows 11 通常可直接运行。
- Linux：AppImage、DEB。
- macOS：DMG（Intel 与 Apple Silicon）。

Windows Release 不生成 MSI 或 NSIS 安装包。未配置签名密钥时仍会生成未签名制品，操作系统可能显示安全警告。

## English

`Font Converter` is a cross-platform Tauri 2 + Leptos desktop font converter with a native CLI.
It converts TTF, OTF, and WOFF2 using Google's pinned WOFF2 reference implementation and Brotli.

The CLI accepts files and directories, scans directories recursively, writes outputs beside the
source by default, never overwrites existing outputs, and supports human-readable or JSON output.

```bash
cargo run -p font-converter-cli -- ./fonts
cargo run -p font-converter-cli -- --json -o ./converted ./font.ttf
```

Pushing a `v*` tag creates a draft release with only a portable Windows x64 executable (no MSI or
NSIS installer), plus Linux AppImage/DEB and macOS Intel/Apple Silicon DMGs. The Windows GUI uses
the system WebView2 runtime; Windows 10 1803+ and Windows 11 normally include it.

## License and attribution

This project is available under either the MIT License or the Apache License 2.0.

Google WOFF2 and Brotli are distributed under the MIT License; their source and license files are
kept in `vendor/woff2`.
