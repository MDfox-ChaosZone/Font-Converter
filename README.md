# Font Converter

[English](#overview) · [简体中文](#简介)

<p align="center">
  <img src="assets/screenshots/font-converter-dark-en.png" alt="Font Converter in English dark mode" width="49%">
  <img src="assets/screenshots/font-converter-light-zh.png" alt="Font Converter in Chinese light mode" width="49%">
</p>

## Overview

Font Converter is a cross-platform desktop font conversion tool with both a GUI and CLI. It supports bidirectional conversion between TTF/OTF and WOFF2.

- When converting WOFF2 to TTF/OTF, Font Converter reads the font-type identifier in the WOFF2 header and automatically selects TTF or OTF as the output format.
- The core codec is based on [Google WOFF2](https://github.com/google/woff2). This project upgrades Google WOFF2's originally pinned Brotli `1.0.3` to Brotli `1.2.0`, the latest stable release at the time of integration. Project benchmarks measured an approximately **10%** improvement in TTF/OTF → WOFF2 conversion speed; actual gains vary by font, hardware, and workload.

### Download

Download the desktop app or CLI from the [latest GitHub Release](https://github.com/MDfox-ChaosZone/Font-Converter/releases/latest).

Available artifacts include:

- Windows x64 and ARM64 portable desktop apps and CLI binaries
- Linux x86_64 and ARM64 AppImage, DEB, RPM, and CLI binaries
- macOS Apple Silicon DMG and CLI binaries

### CLI quick start

Download the CLI for your platform, or build it from source. A few common examples:

```bash
font-converter-cli ./fonts
font-converter-cli --mode encode -o ./converted ./fonts
font-converter-cli --mode decode --jobs 2 ./web-fonts
font-converter-cli --dry-run --json ./fonts
```

Useful options:

- `--mode <auto|encode|decode>` selects the conversion direction.
- `--output-dir <DIR>` writes results to that directory, creates it when needed, and preserves relative paths.
- `--existing <skip|error|overwrite>` controls existing outputs; the default is `skip`.
- `--jobs <N>` sets parallelism; `--dry-run` previews work; `--json` emits one machine-readable report.
- JSON reports include a `formatVersion` and stable `errorCode` values for automation.
- Run `font-converter-cli --help` for the complete, version-specific option list.

### Build from source

Requirements: Rust stable, Node.js 22.12 or newer with npm, Git, a platform C/C++ toolchain, and the GUI prerequisites for [Tauri 2](https://v2.tauri.app/start/prerequisites/).

```bash
git clone --recurse-submodules https://github.com/MDfox-ChaosZone/Font-Converter.git
cd Font-Converter

# CLI
cargo build -p font-converter-cli --release

# Desktop app
npm ci --prefix frontend
cargo install --locked tauri-cli
cargo tauri dev
```

If you cloned without submodules, run:

```bash
git submodule update --init --recursive
```

The release CLI is written to `target/release/font-converter-cli` (`.exe` on Windows).

### Project structure

- `core`: shared conversion engine and Google WOFF2/Brotli native build
- `cli`: command-line interface
- `frontend`: Vue 3 and TypeScript user interface built with Vite
- `src-tauri`: Tauri desktop shell
- `shared`: shared types and conversion workflow

### License

Font Converter is licensed under the [MIT License](LICENSE). Third-party license notices are listed in [THIRD_PARTY_LICENSES](THIRD_PARTY_LICENSES).

### Support and sponsorship

If Font Converter is useful to you, you are welcome to leave a tip for the project. Any sponsorship is greatly appreciated.

#### WeChat Pay and Alipay

<p align="center">
  <img src="assets/donate/wechat-pay.png" alt="WeChat Pay QR code" width="32%">
  <img src="assets/donate/alipay.jpg" alt="Alipay QR code" width="32%">
</p>

#### USDT

- Plasma: `0x742fa2ac27c5d3ff0c337b93ad688d39a77da4c8`
- Aptos: `0xb3ba1611884cc1c2d2d970d081f6c24089d363817a772bddab52a0a278c6ffef`

<p align="center">
  <img src="assets/donate/usdt-plasma.jpg" alt="USDT deposit QR code on Plasma" width="32%">
  <img src="assets/donate/usdt-aptos.jpg" alt="USDT deposit QR code on Aptos" width="32%">
</p>

When transferring USDT,please verify both the network and wallet address. 

---

## 简介

Font Converter 是一款跨桌面平台的字体格式转换工具，同时提供 GUI 和 CLI，支持 TTF/OTF 与 WOFF2 双向转换。

- 将 WOFF2 转换为 TTF/OTF 时，Font Converter 会读取 WOFF2 文件头中的字体类型标识，并自动选择 TTF 或 OTF 作为输出格式。
- 核心编解码算法基于 [Google WOFF2](https://github.com/google/woff2)。本项目将 Google WOFF2 原先固定的 Brotli `1.0.3` 升级为集成时最新的稳定版 Brotli `1.2.0`。项目基准测试显示，TTF/OTF → WOFF2 转换速度提升约 **10%**；实际提升会因字体、硬件和任务类型而异。

### 下载

请前往[最新 GitHub Release](https://github.com/MDfox-ChaosZone/Font-Converter/releases/latest)下载桌面应用或 CLI。

当前发布制品包括：

- Windows x64、ARM64 便携版桌面应用与 CLI
- Linux x86_64、ARM64 的 AppImage、DEB、RPM 与 CLI
- macOS Apple Silicon 的 DMG 与 CLI

### CLI 快速上手

下载对应平台的 CLI，或从源码构建。常用示例：

```bash
font-converter-cli ./fonts
font-converter-cli --mode encode -o ./converted ./fonts
font-converter-cli --mode decode --jobs 2 ./web-fonts
font-converter-cli --dry-run --json ./fonts
```

常用参数：

- `--mode <auto|encode|decode>` 选择转换方向。
- `--output-dir <DIR>` 输出到指定目录、按需自动创建，并保留相对路径。
- `--existing <skip|error|overwrite>` 控制已有文件；默认值为 `skip`。
- `--jobs <N>` 设置并行数；`--dry-run` 仅预览；`--json` 输出一份机器可读报告。
- JSON 报告包含 `formatVersion` 和稳定的 `errorCode`，便于自动化处理。
- 运行 `font-converter-cli --help` 查看当前版本的完整参数。

### 从源码构建

需要 Rust stable、Node.js 22.12 或更新版本及 npm、Git、平台对应的 C/C++ 工具链，以及 [Tauri 2](https://v2.tauri.app/start/prerequisites/) 的 GUI 构建依赖。

```bash
git clone --recurse-submodules https://github.com/MDfox-ChaosZone/Font-Converter.git
cd Font-Converter

# CLI
cargo build -p font-converter-cli --release

# 桌面应用
npm ci --prefix frontend
cargo install --locked tauri-cli
cargo tauri dev
```

如果克隆时没有初始化子模块，请运行：

```bash
git submodule update --init --recursive
```

Release 模式的 CLI 位于 `target/release/font-converter-cli`（Windows 为 `.exe`）。

### 项目结构

- `core`：共享转换引擎及 Google WOFF2/Brotli 原生构建
- `cli`：命令行界面
- `frontend`：使用 Vite 构建的 Vue 3 + TypeScript 用户界面
- `src-tauri`：Tauri 桌面外壳
- `shared`：共享类型与转换流程

### 许可证

Font Converter 使用 [MIT License](LICENSE)。第三方许可证声明见 [THIRD_PARTY_LICENSES](THIRD_PARTY_LICENSES)。

### 支持与赞助

如果 Font Converter 对你有帮助，欢迎给这个项目点心。任何赞助都不胜感激。

#### 微信支付与支付宝

<p align="center">
  <img src="assets/donate/wechat-pay.png" alt="微信支付收款码" width="32%">
  <img src="assets/donate/alipay.jpg" alt="支付宝收款码" width="32%">
</p>

#### USDT

- Plasma：`0x742fa2ac27c5d3ff0c337b93ad688d39a77da4c8`
- Aptos：`0xb3ba1611884cc1c2d2d970d081f6c24089d363817a772bddab52a0a278c6ffef`

<p align="center">
  <img src="assets/donate/usdt-plasma.jpg" alt="Plasma 网络 USDT 充值二维码" width="32%">
  <img src="assets/donate/usdt-aptos.jpg" alt="Aptos 网络 USDT 充值二维码" width="32%">
</p>

转账 USDT 时，请同时核对网络和钱包地址。
