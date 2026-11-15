# GPUI 网页版

用浏览器打开站点，点 **Choose font** 选择 TTF、OTF 或 WOFF2，转换完成后点 **Download**。转换在本机 Web Worker 中进行。网页版每次处理一个字体。

## 构建和预览

在仓库根目录执行以下命令。需要 Python 3、Rust、WASI SDK（已测 25.0）和 wasm-bindgen CLI 0.2.130。

```sh
git submodule update --init --recursive
rustup target add wasm32-unknown-unknown
python web/build.py --sdk PATH_TO_WASI_SDK --bindgen PATH_TO_WASM_BINDGEN
python -m http.server 8137 --bind 127.0.0.1 --directory web/dist
```

打开 <http://127.0.0.1:8137/>。部署时可将 `web/dist/` 的内容上传至支持 WebAssembly 的 HTTPS 静态主机。

本地服务运行时，可执行浏览器往返转换测试。

```sh
python -m pip install playwright pillow fonttools
python -m playwright install chromium
python web/check_browser.py --measure
```
