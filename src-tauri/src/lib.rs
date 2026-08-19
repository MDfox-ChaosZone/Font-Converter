mod batch;
mod commands;

use batch::BatchManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(BatchManager::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::pick_files,
            commands::pick_folder,
            commands::collect_inputs,
            commands::start_conversion,
            commands::cancel_conversion,
            commands::open_output_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Font Converter");
}
