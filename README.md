# Font Converter

## 简介

Font Converter 是一个跨桌面平台字体格式转换工具,支持 TTF/OTF 与 WOFF2 相互转换
- 将 WOFF2 转换为 TTF/OTF 时，程序会根据 WOFF2 文件中的 SFNT flavor 信息自动选择转换为 TTF 或 OTF。
- 使用 Rust、Tauri 2 和 Leptos 构建
- 同时提供 GUI 和 CLI。



## 发布版本

当前正式发布版本为 `v1.0`。`main` 分支使用 Google Brotli `v1.2.0`。请前往[最新 Release](https://github.com/MDfox-ChaosZone/Font-Converter/releases/latest)下载 GUI 或 CLI 制品。

Windows 图形界面版本依赖系统 WebView2。未配置代码签名证书时，Windows 和 macOS 制品可能显示操作系统安全提示。

## CLI使用

构建并运行：

```bash
cargo run -p font-converter-cli -- ./fonts
cargo run -p font-converter-cli -- -o ./converted ./font.ttf ./font.otf
cargo run -p font-converter-cli -- --json ./fonts
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

CLI 使用退出码区分成功（`0`）、转换失败（`1`）、参数错误或没有可转换输入（`2`）以及 Ctrl+C（`130`）。


## 开发环境

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

- Windows：Microsoft C++ Build Tools 和 WebView2；
- macOS：Xcode Command Line Tools；
- Debian/Ubuntu Linux：`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf rpm`。

启动 GUI：

```bash
cargo tauri dev
```

检查和测试：

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy -p font-converter-shared -p font-converter-core -p font-converter-cli -p font-converter --all-targets -- -D warnings
cargo check -p font-converter-frontend --target wasm32-unknown-unknown
trunk build --config frontend/Trunk.toml --release
```

核心转换代码位于 `core`，GUI 和 CLI 均依赖它。Google WOFF2 的 C ABI 只在 `core/native/woff2_wrapper.cc` 和 `core/src/converter.rs` 中适配。真实字体测试可以通过 `FONT_CONVERTER_TEST_FONT=/path/to/font.ttf` 和 `FONT_CONVERTER_TEST_OTF=/path/to/font.otf` 启用。

## 许可证与致谢

本项目采用 MIT License 或 Apache License 2.0，二者任选其一。Google WOFF2 和 Brotli 使用 MIT License，相关源码和许可证文件保存在 `vendor/woff2`。
