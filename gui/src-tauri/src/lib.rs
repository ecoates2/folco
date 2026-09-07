mod dto;
mod state;
mod theme;

use dto::{FolderIconBaseDto, PlatformSizeSpecDto, SvgFolderIconBaseDto};
use state::AppState;
use theme::StartupTheme;

/// Caches the frontend's resolved theme so the next launch can create the window
/// with a matching background colour.
#[tauri::command]
fn set_startup_theme(app: tauri::AppHandle, theme: StartupTheme) -> Result<(), String> {
    theme::write(&app, theme)
}

#[tauri::command]
fn get_folder_icon_base(
    state: tauri::State<AppState>,
) -> Result<Option<FolderIconBaseDto>, String> {
    let Some(base) = state.get_folder_icon_base()? else {
        return Ok(None);
    };
    FolderIconBaseDto::try_from(&base)
        .map(Some)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_folder_icon_svg(
    state: tauri::State<AppState>,
) -> Result<Option<SvgFolderIconBaseDto>, String> {
    Ok(state
        .get_folder_icon_svg()?
        .map(|(svg, surface_color)| SvgFolderIconBaseDto { svg, surface_color }))
}

#[tauri::command]
fn get_platform_icon_sizes() -> PlatformSizeSpecDto {
    folco_core::get_platform_icon_sizes()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_prevent_default::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_folder_icon_base,
            get_folder_icon_svg,
            get_platform_icon_sizes,
            set_startup_theme
        ])
        .setup(|app| {
            // The `main` window is declared with `"create": false` so it can be built
            // here with a background colour matching the theme the user last saw.
            let mut window_config = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == "main")
                .cloned()
                .ok_or("missing `main` window configuration")?;

            window_config.background_color = Some(theme::read(app.handle()).background_color());
            tauri::WebviewWindowBuilder::from_config(app.handle(), &window_config)?.build()?;

            // Register the updater plugin. It stays inert until `plugins.updater`
            // (pubkey + endpoints) and `bundle.createUpdaterArtifacts` are
            // configured -- see the release setup notes.
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            // #[cfg(debug_assertions)]
            // {
            //     let window = app.get_webview_window("main").unwrap();
            //     window.open_devtools();
            // }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
