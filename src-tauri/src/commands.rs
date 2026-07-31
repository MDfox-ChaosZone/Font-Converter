use fontbridge_shared::{QueueItem, ScanResult};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::{batch::BatchManager, scanner};

#[tauri::command]
pub fn pick_files(app: AppHandle) -> Vec<String> {
    app.dialog()
        .file()
        .add_filter("Supported fonts", &["ttf", "otf", "woff2"])
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

#[tauri::command(rename_all = "camelCase")]
pub fn collect_inputs(paths: Vec<String>, output_directory: Option<String>) -> ScanResult {
    scanner::collect(
        &paths,
        output_directory.as_deref().map(std::path::Path::new),
    )
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

#[tauri::command(rename_all = "camelCase")]
pub fn open_output_folder(output_path: String) -> Result<(), String> {
    let directory = output_parent(Path::new(&output_path))?;

    #[cfg(target_os = "windows")]
    let mut command = std::process::Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");

    command
        .arg(&directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Cannot open output folder: {error}"))
}

fn output_parent(output_path: &Path) -> Result<PathBuf, String> {
    if !output_path.is_file() {
        return Err("The converted output file no longer exists".into());
    }
    output_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "The output file has no parent folder".into())
}

#[cfg(test)]
mod tests {
    use super::output_parent;

    #[test]
    fn output_parent_requires_an_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("font.woff2");
        std::fs::write(&output, b"fixture").unwrap();

        assert_eq!(output_parent(&output).unwrap(), directory.path());
        assert!(output_parent(&directory.path().join("missing.woff2")).is_err());
    }
}
