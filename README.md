# Font Converter

## 简介

Font Converter 是一个跨桌面平台字体格式转换工具,支持 TTF/OTF 与 WOFF2 相互转换
- 将 WOFF2 转换为 TTF/OTF 时，程序会根据 WOFF2 文件中的 SFNT flavor 信息自动选择转换为 TTF 或 OTF。
- 使用 Rust、Tauri 2 和 Leptos 构建
- 同时提供 GUI 和 CLI。



## 发布版本

当前项目版本为 `v1.1.0`，CLI 支持转换方向、已有文件策略、dry-run、严格模式、相对目录输出和并行转换。`main` 分支使用 Google Brotli `v1.2.0`。请前往[最新 Release](https://github.com/MDfox-ChaosZone/Font-Converter/releases/latest)下载 GUI 或 CLI 制品；如果 Release 页面尚未提供 `v1.1.0`，可以按照下文说明从源码构建。

Windows 图形界面版本依赖系统 WebView2。未配置代码签名证书时，Windows 和 macOS 制品可能显示操作系统安全提示。

## CLI 使用

CLI 有两种使用方式，选择其中一种即可：

- 普通用户可以从 [Releases](https://github.com/MDfox-ChaosZone/Font-Converter/releases/latest) 下载已经编译好的 CLI，无需安装 Rust；
- 开发者或希望体验最新源码的用户，可以克隆仓库后使用 Cargo 自行编译。

### 下载发布版直接运行

在 Release 页面根据操作系统和 CPU 架构下载对应的 CLI 制品。下载的文件已经完成编译，可以直接运行，不需要再执行 `cargo run`。

Windows x64 示例：

```powershell
.\Font-Converter-cli-windows-x64.exe --help
.\Font-Converter-cli-windows-x64.exe .\fonts
```

Linux x86_64 示例：

```bash
chmod +x Font-Converter-cli-linux-x86_64
./Font-Converter-cli-linux-x86_64 --help
./Font-Converter-cli-linux-x86_64 ./fonts
```

本文 CLI 文档统一以 `v1.1.0` 为准。下载后请通过 `--version` 和 `--help` 确认版本及其支持的参数；旧版 CLI 不支持本文列出的全部选项。

### 从源码构建并运行

从源码构建 CLI 需要 Rust stable、Git、平台对应的 C/C++ 编译工具，以及完整的 Git 子模块。克隆仓库并初始化子模块：

```bash
git clone --recurse-submodules https://github.com/MDfox-ChaosZone/Font-Converter.git
cd Font-Converter
```

如果仓库之前不是使用 `--recurse-submodules` 克隆的，请执行：

```bash
git submodule update --init --recursive
```

使用 Cargo 编译当前源码并立即运行：

```bash
cargo run -p font-converter-cli -- ./fonts
cargo run -p font-converter-cli -- --mode encode -o ./converted ./fonts
cargo run -p font-converter-cli -- --mode decode --jobs 2 ./web-fonts
cargo run -p font-converter-cli -- --dry-run --json ./fonts
```

`cargo run` 会先编译源码，再启动生成的 CLI。命令中单独的 `--` 用于分隔 Cargo 参数和传递给 `font-converter-cli` 的参数。

如果要生成经过优化、可重复使用的 CLI 二进制，请执行：

```bash
cargo build -p font-converter-cli --release
```

构建结果位于：

- Windows：`target/release/font-converter-cli.exe`
- Linux/macOS：`target/release/font-converter-cli`

例如在 Linux 上运行：

```bash
./target/release/font-converter-cli --help
./target/release/font-converter-cli --mode encode -o ./converted ./fonts
```

### v1.1 CLI 用法

将以下命令中的 `font-converter-cli` 替换为下载文件的路径、源码构建结果的路径，或者已加入 `PATH` 的命令名：

```text
font-converter-cli [OPTIONS] <PATH>...

Options:
  -o, --output-dir <DIR>       Put generated files in this existing directory, preserving subdirectories
      --mode <MODE>            Conversion direction: auto, encode, or decode [default: auto]
      --existing <POLICY>      Existing output policy: skip, error, or overwrite [default: skip]
      --dry-run                Report planned work without writing output files
      --strict                 Treat scan warnings and skipped items as failures
  -j, --jobs <N>               Concurrent conversions [default: up to 4 logical CPUs]
      --json                   Print one JSON report instead of progress output
  -q, --quiet                  Suppress human-readable progress output
  -h, --help                   Print help
  -V, --version                Print version
```

`--mode encode` 只处理 TTF/OTF → WOFF2，`--mode decode` 只处理 WOFF2 → TTF/OTF，`auto` 保持双向自动识别。指定输出目录后，目录输入的相对层级会保留。`--existing overwrite` 必须显式指定；默认不会覆盖已有文件。

CLI 使用退出码区分成功（`0`）、转换或严格模式失败（`1`）、参数错误或没有可转换输入（`2`）以及 Ctrl+C（`130`）。


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
