mod dto;
mod state;
mod theme;

use std::path::PathBuf;

use dto::{FolderIconBaseDto, PlatformSizeSpecDto, ProgressDto, SvgFolderIconBaseDto};
use folco_core::CustomizationProfile;
use folco_core::progress::progress_channel;
use state::AppState;
use tauri::ipc::Channel;
use tauri::Manager;
use theme::StartupTheme;

/// Caches the frontend's resolved theme so the next launch can create the window
/// with a matching background colour.
#[tauri::command]
fn set_startup_theme(app: tauri::AppHandle, theme: StartupTheme) -> Result<(), String> {
    theme::write(&app, theme)
}

/// Runs `f` against the app state on a blocking thread.
///
/// Sync commands run on the main thread, and these may wait on the context lock
/// while `warm()` builds it — which would freeze the window.
async fn with_state_blocking<T: Send + 'static>(
    app: tauri::AppHandle,
    f: impl FnOnce(&AppState) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_folder_icon_base(app: tauri::AppHandle) -> Result<Option<FolderIconBaseDto>, String> {
    with_state_blocking(app, AppState::get_folder_icon_base).await
}

#[tauri::command]
async fn get_folder_icon_svg(
    app: tauri::AppHandle,
) -> Result<Option<SvgFolderIconBaseDto>, String> {
    with_state_blocking(app, |state| {
        Ok(state
            .get_folder_icon_svg()?
            .map(|svg| SvgFolderIconBaseDto { svg }))
    })
    .await
}

#[tauri::command]
fn get_platform_icon_sizes() -> PlatformSizeSpecDto {
    folco_core::get_platform_icon_sizes()
}

/// Applies a `CustomizationProfile` (JSON) to the system folder icon and writes it
/// to every folder, streaming per-folder progress over `on_progress`.
///
/// Resolves once all folders are processed. Per-folder failures arrive as
/// `folderFailed` events, not as an `Err`.
#[tauri::command]
async fn customize_folders(
    app: tauri::AppHandle,
    folders: Vec<PathBuf>,
    profile_json: String,
    on_progress: Channel<ProgressDto>,
) -> Result<(), String> {
    let profile = CustomizationProfile::from_json(&profile_json)
        .map_err(|e| format!("Invalid customization profile: {e}"))?;

    let (tx, mut rx) = progress_channel(32);
    let forward = tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            if on_progress.send(event).is_err() {
                break;
            }
        }
    });

    let result =
        with_state_blocking(app, move |state| state.customize_folders(folders, profile, tx)).await;
    forward.await.map_err(|e| e.to_string())?;
    result
}

#[tauri::command]
fn experimental_enabled() -> bool {
    folco_core::experimental::enabled_from_env()
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
            customize_folders,
            experimental_enabled,
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

            // Build the customization context off the critical path: the window is
            // already up, and the frontend's first icon request usually lands after
            // this finishes rather than paying for it.
            let warm_handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                if let Err(e) = warm_handle.state::<AppState>().warm() {
                    eprintln!("Failed to warm customization context: {e}");
                }
            });

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
