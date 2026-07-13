mod dto;
mod state;

use dto::{FolderIconBaseDto, PlatformSizeSpecDto};
use state::AppState;
use tauri::Manager;

#[tauri::command]
fn get_folder_icon_base(state: tauri::State<AppState>) -> Result<FolderIconBaseDto, String> {
    let base = state.get_folder_icon_base()?;
    FolderIconBaseDto::try_from(&base)
}

#[tauri::command]
fn get_platform_icon_sizes() -> PlatformSizeSpecDto {
    folco_core::get_platform_icon_sizes().into()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new().expect("Failed to initialize app state");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_prevent_default::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_folder_icon_base,
            get_platform_icon_sizes
        ])
        .setup(|app| {
            // Register the updater plugin. It stays inert until `plugins.updater`
            // (pubkey + endpoints) and `bundle.createUpdaterArtifacts` are
            // configured -- see the release setup notes.
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
