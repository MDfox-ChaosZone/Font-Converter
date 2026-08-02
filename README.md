# Font Converter

[English](#english) · [简体中文](#简体中文)

## 简介

- Font Converter 是一个跨桌面平台的字体格式转换工具, 实现TTF/OTF与WOFF2的相互转换
  - 将WOFF2转换为 TTF/OTF 时,会根据 WOFF2 文件中的 SFNT flavor信息自动选择转换为TTF还是OTF
- 提供桌面 GUI 和命令行 CLI
- 基于 Google WOFF2 参考实现.
- 使用 Rust、Tauri 2 和 Leptos 构建

## 发布版本

| 发布版本 | 标签 | Brotli 版本 | 适用场景 |
| --- | --- | --- | --- |
| 正式版 | `v1.0.0` | Google/WOFF2项目使用的固定Brotli版本 `v1.0.3` | 广经检验，作为默认推荐版本 |
| Alpha 实验版 | `v1.0.0-alpha.1` | 使用Brotli最新版本 `v1.2.0` | 提高TTF/OTF → WOFF2的转换速度 |

在本项目的测试样本和测试环境中，Brotli 1.2.0 版本的 TTF/OTF → WOFF2 平均每轮耗时减少 12.73%，两种版本生成的转换字体哈希完全一致。WOFF2 → TTF/OTF 的解码耗时略有增加，详见下表：

| 任务 | Brotli 1.0.3 | Brotli 1.2.0 | 差异 |
| --- | ---: | ---: | ---: |
| TTF/OTF → WOFF2，平均每轮耗时 | 83.599 秒 | 72.960 秒 | -12.73% |
| WOFF2 → TTF/OTF，平均每轮耗时 | 441.038 ms | 450.320 ms | +2.11% |

以下是测试样本中的字体文件：

| 字体 | 格式 | 文件大小 |
| --- | --- | ---: |
| AlibabaPuHuiTi-3-75-SemiBold | OTF | 7.030 MB |
| AlibabaPuHuiTi-3-75-SemiBold | WOFF2 | 5.481 MB |
| KaTeX_Size1-Regular | TTF | 11.932 KB |
| KaTeX_Size1-Regular | WOFF2 | 5.332 KB |
| LXGWWenKai-Medium | TTF | 25.380 MB |
| LXGWWenKai-Medium | WOFF2 | 8.953 MB |
| MapleMonoNormalNL-Regular | TTF | 241.240 KB |
| MapleMonoNormalNL-Regular | WOFF2 | 65.420 KB |
| NotoColorEmoji-Regular | TTF | 25.112 MB |
| NotoColorEmoji-Regular | WOFF2 | 5.715 MB |

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
