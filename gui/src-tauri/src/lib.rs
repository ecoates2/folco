mod cli;
mod dto;
mod state;
mod theme;

use std::path::PathBuf;

use cli::LaunchDirectories;
use dto::{
    AppMetadataDto, CustomImageDto, CustomImageSupportDto, FolderIconBaseDto, PlatformSizeSpecDto, ProgressDto,
    SvgFolderIconBaseDto,
};
use folco_core::progress::{progress_channel, ProgressSender};
use folco_core::{CustomizationProfile, WriteCapabilities};
use state::AppState;
use tauri::ipc::{Channel, Response};
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
fn get_custom_image_sizes() -> PlatformSizeSpecDto {
    PlatformSizeSpecDto::custom_image()
}

#[tauri::command]
fn get_custom_image_support() -> CustomImageSupportDto {
    let write = WriteCapabilities::current_platform();
    CustomImageSupportDto {
        write,
        extensions: write.custom_image_extensions(),
    }
}

/// Runs a folder operation on the app state, forwarding its progress events to
/// `on_progress` until the operation drops its sender.
async fn run_with_progress(
    app: tauri::AppHandle,
    on_progress: Channel<ProgressDto>,
    op: impl FnOnce(&AppState, ProgressSender) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    let (tx, mut rx) = progress_channel(32);
    let forward = tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            if on_progress.send(event).is_err() {
                break;
            }
        }
    });

    let result = with_state_blocking(app, move |state| op(state, tx)).await;
    forward.await.map_err(|e| e.to_string())?;
    result
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

    run_with_progress(app, on_progress, move |state, tx| {
        state.customize_folders(folders, profile, tx)
    })
    .await
}

/// Restores the system default icon on every folder, streaming progress like
/// [`customize_folders`].
#[tauri::command]
async fn reset_folders(
    app: tauri::AppHandle,
    folders: Vec<PathBuf>,
    on_progress: Channel<ProgressDto>,
) -> Result<(), String> {
    run_with_progress(app, on_progress, move |state, tx| state.reset_folders(folders, tx)).await
}

/// Reads an image file into app state as the custom base image.
#[tauri::command]
async fn load_custom_image(app: tauri::AppHandle, path: PathBuf) -> Result<CustomImageDto, String> {
    with_state_blocking(app, move |state| state.load_custom_image(&path)).await
}

/// Raw bytes of custom image `revision`, so the preview renders exactly what
/// the backend will write.
#[tauri::command]
async fn get_custom_image_bytes(app: tauri::AppHandle, revision: u64) -> Result<Response, String> {
    with_state_blocking(app, move |state| state.custom_image_bytes(revision))
        .await
        .map(Response::new)
}

/// Uses custom image `image_revision` as the folder icon, decorated per the
/// profile, streaming progress like [`customize_folders`].
#[tauri::command]
async fn customize_folders_with_image(
    app: tauri::AppHandle,
    folders: Vec<PathBuf>,
    image_revision: u64,
    profile_json: String,
    on_progress: Channel<ProgressDto>,
) -> Result<(), String> {
    let profile = CustomizationProfile::from_json(&profile_json)
        .map_err(|e| format!("Invalid customization profile: {e}"))?;

    run_with_progress(app, on_progress, move |state, tx| {
        state.customize_folders_with_image(folders, image_revision, profile, tx)
    })
    .await
}

#[tauri::command]
fn experimental_enabled() -> bool {
    folco_core::experimental::enabled_from_env()
}

#[tauri::command]
fn get_app_metadata(app: tauri::AppHandle) -> AppMetadataDto {
    let info = app.package_info();
    AppMetadataDto {
        description: info.description,
        authors: info.authors.split(':').filter(|a| !a.is_empty()).collect(),
        repository: env!("CARGO_PKG_REPOSITORY"),
        debug: cfg!(debug_assertions),
    }
}

/// Folders passed on the command line, to pre-populate the folder list.
#[tauri::command]
fn get_launch_directories(launch: tauri::State<'_, LaunchDirectories>) -> Vec<PathBuf> {
    launch.0.clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launch_directories = LaunchDirectories::from_args();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_prevent_default::init())
        .manage(AppState::new())
        .manage(launch_directories)
        .invoke_handler(tauri::generate_handler![
            get_folder_icon_base,
            get_folder_icon_svg,
            get_custom_image_sizes,
            get_custom_image_support,
            customize_folders,
            customize_folders_with_image,
            load_custom_image,
            get_custom_image_bytes,
            reset_folders,
            experimental_enabled,
            get_launch_directories,
            get_app_metadata,
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
