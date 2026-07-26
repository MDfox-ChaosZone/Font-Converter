# ttf2woff2-GUI

[English](#english) · [简体中文](#简体中文)

## 简体中文

`ttf2woff2-GUI` 是基于 Tauri 2 和 Leptos 的跨平台桌面字体转换器。界面、文件处理和转换流程均使用 Rust 实现，转换核心来自纯 Rust 的
[`0x6b/ttf2woff2`](https://github.com/0x6b/ttf2woff2)。

### 功能

- 拖放一个或多个 TTF 文件或文件夹。
- 原生文件多选和文件夹选择对话框。
- 递归扫描子目录，不跟随目录符号链接。
- 在源字体旁生成同名 `.woff2`，绝不覆盖已有文件。
- 顺序后台转换、逐项进度、错误隔离和批次取消。
- 默认使用 Brotli 质量 11 和单线程编码，优先获得最小且可复现的输出。
- 简体中文/英文界面，语言选择自动保存。
- Windows x64、Linux x64、macOS Intel 和 Apple Silicon 构建。

上游编码器目前只支持带 TrueType outlines 的 TTF，不支持 OTF/CFF、WOFF1 或反向转换。

### 开发环境

通用依赖：

```text
Rust stable（包含 wasm32-unknown-unknown target）
Trunk
Tauri CLI 2
```

安装 Rust 工具：

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
cargo install --locked tauri-cli
```

各平台还需要：

- Windows：Microsoft C++ Build Tools 和 WebView2。
- macOS：Xcode Command Line Tools。
- Debian/Ubuntu Linux：`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`。

启动开发版本：

```bash
cargo tauri dev
```

运行检查：

```bash
cargo fmt --all -- --check
cargo test -p ttf2woff2-gui-shared -p ttf2woff2-gui
cargo clippy -p ttf2woff2-gui-shared -p ttf2woff2-gui --all-targets -- -D warnings
cargo check -p ttf2woff2-gui-frontend --target wasm32-unknown-unknown
trunk build --config frontend/Trunk.toml --release
```

可通过 `TTF2WOFF2_TEST_FONT=/path/to/font.ttf` 启用真实字体的确定性转换测试。CI 使用上游 `v0.13.2`
中带 OFL 许可的 Warpnine Sans fixture。

### 上游更新

后端的 `src-tauri/src/converter.rs` 是唯一直接使用 `ttf2woff2` API 的文件。Dependabot 每日检查 Cargo
正式版本并创建更新 PR；合并前需通过真实字体测试、WebAssembly 构建和四种桌面目标构建。

手动更新时：

```bash
# 先修改 src-tauri/Cargo.toml 中的精确版本，然后更新锁文件
cargo update -p ttf2woff2 --precise <version>
```

如果升级到新版本，还需同步修改 `src-tauri/Cargo.toml` 中的精确版本以及 CI fixture 的版本标签。
编码器 API 如果发生变化，只需适配 `src-tauri/src/converter.rs`，GUI 和批处理层不直接依赖上游类型。

### 编码参数

当前版本固定使用 Brotli 质量 `11`、`glyf/loca` 转换和单线程压缩：

- 质量 `11`：上游支持范围为 `0–11`；数值越高，通常输出越小，但编码耗时越长。
- `glyf/loca` 转换：按 WOFF2 规范重组 TrueType 字形表，通常能进一步缩小文件。
- 单线程：保证相同输入和版本产生确定性输出；任务队列也会逐个处理字体，控制内存占用。

首版暂不在界面中暴露这些高级参数，以保持输出行为稳定。

### 发布与签名

推送 `v*` 标签会创建草稿 GitHub Release，并构建：

- Windows：MSI、NSIS
- Linux：AppImage、DEB
- macOS：APP、DMG（Intel 与 Apple Silicon）

未配置签名密钥时仍会生成测试制品，但操作系统可能显示安全警告。正式公开发布前，应配置 Windows
代码签名证书（`WINDOWS_CERTIFICATE` 为 Base64 PFX、`WINDOWS_CERTIFICATE_PASSWORD`，以及可选
`WINDOWS_TIMESTAMP_URL`），以及 `APPLE_CERTIFICATE`、`APPLE_SIGNING_IDENTITY`、`APPLE_ID`、
`APPLE_PASSWORD` 和 `APPLE_TEAM_ID` 等 GitHub Secrets 完成 macOS 签名与公证。缺少 Windows
证书时，签名脚本会安全退出并保留未签名制品。

## English

`ttf2woff2-GUI` is a cross-platform Tauri 2 + Leptos desktop application that converts TrueType
fonts to WOFF2. The UI, filesystem workflow, and conversion orchestration are written in Rust. It
uses the pure-Rust [`0x6b/ttf2woff2`](https://github.com/0x6b/ttf2woff2) encoder.

Drag files or recursively scanned folders into the application, review the queue, then start a safe
background conversion. Outputs are written beside their source fonts and existing WOFF2 files are
never overwritten. Encoding uses Brotli quality 11, the WOFF2 `glyf/loca` transform, and a
single deterministic compression thread. See the Chinese section above for development, testing,
release, and signing commands.

## License and attribution

This project is available under either the MIT License or the Apache License 2.0.

The upstream `ttf2woff2` crate is copyright its contributors and is also distributed under
`MIT OR Apache-2.0`. The Warpnine Sans fixture downloaded only during CI is covered by the SIL Open
Font License in the upstream repository. No upstream fixture is redistributed in application
packages.
