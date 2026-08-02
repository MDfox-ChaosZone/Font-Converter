# Font Converter

[English](#english) · [简体中文](#简体中文)

## 简体中文

Font Converter 是一个跨桌面平台的字体格式转换工具, 实现TTF/OTF与WOFF2的相互转换.提供桌面 GUI 和命令行 CLI,基于 Google WOFF2 参考实现.
是一个使用 Rust、Tauri 2 和 Leptos 构建的跨平台字体转换工具，

### 版本与算法

项目提供两条发布线：

| 发布线 | 标签 | Brotli 版本 | 适用场景 |
| --- | --- | --- | --- |
| 正式版 | `v1.0.0` | Google 固定的 `v1.0.3` | 稳定使用，作为默认推荐版本 |
| Alpha 实验版 | `v1.0.0-alpha.1` | 更新的 `v1.2.0` | 测试新算法性能，可能包含实验性变化 |

两条发布线使用相同的 WOFF2 转换逻辑和相同的字体轮廓处理方式。正式版保留 Google WOFF2 所使用的 Brotli 1.0.3；Alpha 版只替换 Brotli 子模块，用于评估新版算法，不应在未经验证的生产流程中盲目替换正式版。

在本项目的测试样本和测试环境中，Brotli 1.2.0 版本的 TTF/OTF → WOFF2 平均每轮耗时减少 12.73%，两种版本生成的转换字体哈希完全一致。WOFF2 → TTF/OTF 的解码耗时略有增加，详见下表：

| 任务 | Brotli 1.0.3 | Brotli 1.2.0 | 差异 |
| --- | ---: | ---: | ---: |
| TTF/OTF → WOFF2，平均每轮耗时 | 83.599 秒 | 72.960 秒 | -12.73% |
| WOFF2 → TTF/OTF，平均每轮耗时 | 441.038 ms | 450.320 ms | +2.11% |

以下是测试样本中的字体文件大小。实际大小会随字体轮廓、字形数量和字体表结构变化：

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

### 功能

- TTF、OTF 与 WOFF2 双向转换，并自动识别方向。
- 读取 WOFF2 文件中的 SFNT flavor：TrueType 输出 `.ttf`，CFF/OpenType 输出 `.otf`。
- 支持文件和目录输入，目录会递归扫描。
- 默认将结果写入源文件所在目录，也可以指定统一的输出目录。
- 不覆盖已有输出文件；CLI 支持人类可读进度、静默模式和 JSON 报告。
- GUI 后台并行转换，最多同时处理 4 个字体。
- GUI 支持浅色、深色和跟随系统三种主题模式。
- 暂不支持 WOFF1 和字体集合。

### CLI

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

### 下载与发布制品

GitHub Actions 会在推送 `v*` 标签时自动构建并发布 [GitHub Release](https://github.com/MDfox-ChaosZone/Font-Converter/releases/latest)。Release 页面会为每个制品提供直接下载链接，并说明其适用平台：

- Windows x64：便携版 GUI 和 CLI，适用于 Intel/AMD 64 位 Windows；
- Windows ARM64：便携版 GUI 和 CLI，适用于 ARM64 Windows；
- Linux x86_64：AppImage、DEB、RPM，以及 CLI 二进制；
- Linux ARM64：AppImage、DEB、RPM，以及 CLI 二进制；
- macOS Apple Silicon：arm64 DMG 和 CLI 二进制；
- macOS Intel：不发布任何制品；
- CLI 二进制覆盖上述全部原生构建目标，适合批处理和无图形界面环境。

带有连字符的标签（例如 `v1.0.0-alpha.1`）会自动标记为 GitHub Pre-release。Windows 便携版通常可在 Windows 10 1803+ 和 Windows 11 上直接运行，但仍依赖系统 WebView2 运行时。未配置签名密钥时，macOS 和 Windows 制品可能显示操作系统安全提示。

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

### 许可证与致谢

本项目采用 MIT License 或 Apache License 2.0，二者任选其一。Google WOFF2 和 Brotli 使用 MIT License，相关源码和许可证文件保存在 `vendor/woff2`。

## English

Font Converter is a cross-platform font conversion tool built with Rust, Tauri 2, and Leptos. It provides both a desktop GUI and a native command-line interface for converting TTF, OTF, and WOFF2 fonts in either direction.

### Release channels and algorithms

The project publishes two release lines:

| Channel | Tag | Brotli version | Purpose |
| --- | --- | --- | --- |
| Stable | `v1.0.0` | Google-pinned `v1.0.3` | Recommended for stable use |
| Alpha | `v1.0.0-alpha.1` | Updated `v1.2.0` | New algorithm performance evaluation |

Both lines share the same WOFF2 conversion and font-outline handling. Stable releases keep the Brotli 1.0.3 version used by Google WOFF2. Alpha releases update only the Brotli submodule so the newer algorithm can be evaluated without changing the stable default.

In the supplied benchmark samples and test environment, Brotli 1.2.0 reduced average TTF/OTF → WOFF2 conversion time by 12.73%, and the converted font hashes were identical between the two versions. Decode performance changed only slightly:

| Task | Brotli 1.0.3 | Brotli 1.2.0 | Difference |
| --- | ---: | ---: | ---: |
| TTF/OTF → WOFF2, average round | 83.599 s | 72.960 s | -12.73% |
| WOFF2 → TTF/OTF, average round | 441.038 ms | 450.320 ms | +2.11% |

Representative sample file sizes:

| Font | Format | File size |
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

### Features

- Bidirectional TTF, OTF, and WOFF2 conversion with automatic direction detection.
- SFNT flavor detection for WOFF2 files: TrueType becomes `.ttf`, while CFF/OpenType becomes `.otf`.
- Recursive directory scanning and multiple input files.
- Source-directory output by default, or a shared output directory selected by the user.
- No overwriting of existing outputs; the CLI supports human-readable, quiet, and JSON modes.
- Background GUI conversion with up to four concurrent fonts.
- Light, dark, and system-following GUI themes.
- WOFF1 and font collections are not currently supported.

### CLI

```bash
cargo run -p font-converter-cli -- ./fonts
cargo run -p font-converter-cli -- --json -o ./converted ./font.ttf
```

The CLI uses exit code `0` for success, `1` for conversion failures, `2` for invalid arguments or no convertible input, and `130` for Ctrl+C.

### Downloads and CI releases

Pushing a `v*` tag starts GitHub Actions and publishes a [GitHub Release](https://github.com/MDfox-ChaosZone/Font-Converter/releases/latest) with direct links and platform guidance for every asset: Windows x64 and ARM64 portable GUI/CLI binaries, Linux x86_64 and ARM64 AppImage/DEB/RPM packages plus CLI binaries, and an Apple Silicon macOS DMG plus CLI binary. No Intel macOS artifacts are published. Tags containing a hyphen, such as `v1.0.0-alpha.1`, are published as prereleases. The Windows GUI uses the system WebView2 runtime.

### License

Available under either the MIT License or the Apache License 2.0. Google WOFF2 and Brotli are distributed under the MIT License; their source and license files are kept in `vendor/woff2`.
