use font_converter_shared::{FolderConversionMode, ProgressEvent, QueueItem, ScanResult};
use js_sys::{Function, Promise};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};
use wasm_bindgen_futures::{JsFuture, spawn_local};

#[wasm_bindgen(module = "/src/tauri.js")]
extern "C" {
    #[wasm_bindgen(js_name = invokeCommand)]
    fn invoke_command(command: &str, args: JsValue) -> Promise;

    #[wasm_bindgen(js_name = listenProgress)]
    fn listen_progress(callback: &Function) -> Promise;

    #[wasm_bindgen(js_name = listenDragDrop)]
    fn listen_drag_drop(callback: &Function) -> Promise;
}

#[derive(Serialize)]
struct EmptyArgs {}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PathsArgs {
    paths: Vec<String>,
    output_directory: Option<String>,
    folder_conversion_mode: Option<FolderConversionMode>,
}

#[derive(Serialize)]
struct StartArgs {
    items: Vec<QueueItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CancelArgs {
    batch_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OutputPathArgs {
    output_path: String,
}

pub async fn pick_files() -> Result<Vec<String>, String> {
    invoke("pick_files", &EmptyArgs {}).await
}

pub async fn pick_folder() -> Result<Vec<String>, String> {
    invoke("pick_folder", &EmptyArgs {}).await
}

pub async fn collect_inputs(
    paths: Vec<String>,
    output_directory: Option<String>,
    folder_conversion_mode: Option<FolderConversionMode>,
) -> Result<ScanResult, String> {
    invoke(
        "collect_inputs",
        &PathsArgs {
            paths,
            output_directory,
            folder_conversion_mode,
        },
    )
    .await
}

pub async fn start_conversion(items: Vec<QueueItem>) -> Result<String, String> {
    invoke("start_conversion", &StartArgs { items }).await
}

pub async fn cancel_conversion(batch_id: String) -> Result<bool, String> {
    invoke("cancel_conversion", &CancelArgs { batch_id }).await
}

pub async fn open_output_folder(output_path: String) -> Result<(), String> {
    invoke("open_output_folder", &OutputPathArgs { output_path }).await
}

pub fn setup_progress_listener(callback: impl Fn(ProgressEvent) + 'static) {
    let closure = Closure::<dyn Fn(JsValue)>::new(move |value| {
        if let Ok(event) = serde_wasm_bindgen::from_value(value) {
            callback(event);
        }
    });
    spawn_local(async move {
        let result = JsFuture::from(listen_progress(
            closure.as_ref().unchecked_ref::<Function>(),
        ))
        .await;
        if result.is_ok() {
            closure.forget();
        }
    });
}

pub fn setup_drag_drop_listener(callback: impl Fn(bool, Vec<String>) + 'static) {
    let closure = Closure::<dyn Fn(JsValue)>::new(move |value| {
        if let Ok(event) = serde_wasm_bindgen::from_value::<DragDropEvent>(value) {
            let dragging = matches!(event.kind.as_str(), "enter" | "over");
            let paths = if event.kind == "drop" {
                event.paths
            } else {
                Vec::new()
            };
            callback(dragging, paths);
        }
    });
    spawn_local(async move {
        let result = JsFuture::from(listen_drag_drop(
            closure.as_ref().unchecked_ref::<Function>(),
        ))
        .await;
        if result.is_ok() {
            closure.forget();
        }
    });
}

#[derive(Deserialize)]
struct DragDropEvent {
    kind: String,
    #[serde(default)]
    paths: Vec<String>,
}

async fn invoke<A, R>(command: &str, args: &A) -> Result<R, String>
where
    A: Serialize,
    R: DeserializeOwned,
{
    let args = serde_wasm_bindgen::to_value(args).map_err(|error| error.to_string())?;
    let value = JsFuture::from(invoke_command(command, args))
        .await
        .map_err(js_error)?;
    serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
}

fn js_error(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| format!("Tauri command failed: {value:?}"))
}
