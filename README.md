# ttf2woff2-GUI

[English](#english) · [简体中文](#简体中文)

## 简体中文

`ttf2woff2-GUI` 是基于 Tauri 2 和 Leptos 的跨平台桌面字体转换器。界面、文件处理和转换流程均使用 Rust 实现，转换核心采用 Google 官方
[`google/woff2`](https://github.com/google/woff2) C++ 参考实现及其固定版本的 Brotli 子模块。

### 功能

- 拖放一个或多个 TTF、OTF、WOFF2 文件或文件夹。
- 原生文件多选和文件夹选择对话框。
- 递归扫描子目录，不跟随目录符号链接。
- 自动执行 TTF→WOFF2、OTF→WOFF2、WOFF2→TTF 或 WOFF2→OTF。
- 在源字体旁生成转换结果，绝不覆盖已有文件。
- 顺序后台转换、逐项进度、错误隔离和批次取消。
- 任务状态实时更新，转换完成后显示输入与输出文件大小，并可逐项移除队列条目。
- 默认使用 Brotli 质量 11 和单线程编码，优先获得最小且可复现的输出。
- 简体中文/英文界面，语言选择自动保存。
- Windows x64、Linux x64、macOS Intel 和 Apple Silicon 构建。

WOFF2 解码时会读取文件头中的 SFNT flavor：TrueType 输出 `.ttf`，CFF/OpenType 输出 `.otf`。
WOFF2 不保存原始文件扩展名，因此带 TrueType outlines、但原先使用 `.otf` 扩展名的字体会恢复为
`.ttf`。暂不支持 WOFF1 和字体集合。

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
git submodule update --init --recursive
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

可通过 `TTF2WOFF2_TEST_FONT=/path/to/font.ttf` 和
`TTF2WOFF2_TEST_OTF=/path/to/font.otf` 启用真实字体的双向转换测试。CI 从固定提交下载带
OFL 许可的 Abel TTF 和 Source Code Pro OTF 测试字体。

### 上游更新

上游源码以 Git submodule 固定在 `vendor/woff2`。`src-tauri/native/woff2_wrapper.cc` 是唯一直接调用
Google C++ API 的文件，`src-tauri/src/converter.rs` 只依赖稳定的内部 C ABI。Dependabot 每日检查
submodule 更新；合并前需通过真实字体测试、WebAssembly 构建和四种桌面目标构建。

手动更新时：

```bash
git submodule update --init --recursive
git -C vendor/woff2 fetch origin
git -C vendor/woff2 checkout <需要验证的提交>
git -C vendor/woff2 submodule update --init --recursive
cargo test -p ttf2woff2-gui
```

确认四平台 CI 均通过后，提交新的 submodule 指针。若上游 API 发生变化，只需适配
`src-tauri/native/woff2_wrapper.cc`；GUI、扫描器和批处理层不直接依赖上游类型。普通克隆必须使用
`git clone --recurse-submodules`，已有克隆则运行上述初始化命令。

### 编码参数

编码到 WOFF2 时固定使用 Brotli 质量 `11`、适用时启用 `glyf/loca` 转换，并采用单线程压缩：

- 质量 `11`：上游支持范围为 `0–11`；数值越高，通常输出越小，但编码耗时越长。
- `glyf/loca` 转换：按 WOFF2 规范重组 TrueType 字形表；CFF/OTF 没有这些表，会自动跳过。
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

`ttf2woff2-GUI` is a cross-platform Tauri 2 + Leptos desktop application that converts TTF and OTF
fonts to WOFF2 and decompresses WOFF2 back to its TrueType or CFF/OpenType SFNT form. The UI,
filesystem workflow, and conversion orchestration are written in Rust. The
encoder is Google's official C++ [`google/woff2`](https://github.com/google/woff2) reference
implementation, pinned with its Brotli dependency as a Git submodule.

Drag files or recursively scanned folders into the application, review the automatically detected
conversion direction, then start a safe background conversion. Outputs are written beside their
source fonts and existing files are never overwritten. Encoding uses Brotli quality 11, the WOFF2
`glyf/loca` transform when applicable, and a single deterministic compression thread. See the
Chinese section above for development, testing, release, and signing commands.

## License and attribution

This project is available under either the MIT License or the Apache License 2.0.

Google WOFF2 and Brotli are distributed under the MIT License; their source and license files are
kept in `vendor/woff2`. The Abel and Source Code Pro fixtures downloaded only during CI are covered
by the SIL Open Font License. No test font is redistributed in application packages.
