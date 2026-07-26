use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use ttf2woff2_gui_shared::{QueueItem, ScanResult};

use crate::{batch::BatchManager, scanner};

#[tauri::command]
pub fn pick_files(app: AppHandle) -> Vec<String> {
    app.dialog()
        .file()
        .add_filter("TrueType fonts", &["ttf"])
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|path| path.into_path().ok())
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

#[tauri::command]
pub fn pick_folder(app: AppHandle) -> Vec<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|path| path.into_path().ok())
        .map(|path| vec![path.to_string_lossy().into_owned()])
        .unwrap_or_default()
}

#[tauri::command]
pub fn collect_inputs(paths: Vec<String>) -> ScanResult {
    scanner::collect(&paths)
}

#[tauri::command]
pub fn start_conversion(
    app: AppHandle,
    manager: State<'_, BatchManager>,
    items: Vec<QueueItem>,
) -> Result<String, String> {
    if items.is_empty() {
        return Err("No conversion items were supplied".into());
    }
    Ok(manager.start(app, items))
}

#[tauri::command(rename_all = "camelCase")]
pub fn cancel_conversion(manager: State<'_, BatchManager>, batch_id: String) -> bool {
    manager.cancel(&batch_id)
}
