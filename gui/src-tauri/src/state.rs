use std::sync::Mutex;

use folco_core::{CustomizationContext, CustomizationContextBuilder, FolderIconBase, SurfaceColor};

/// Wrapper to make `CustomizationContext` usable in Tauri managed state.
///
/// `CustomizationContext` contains platform COM pointers (e.g., `IKnownFolderManager`
/// on Windows) that are not `Send`/`Sync`. Since we always access it behind a `Mutex`,
/// this is safe.
struct SendableContext(CustomizationContext);

// SAFETY: Access is always serialized through a Mutex.
unsafe impl Send for SendableContext {}
unsafe impl Sync for SendableContext {}

/// Tauri managed state wrapping the `CustomizationContext`.
pub struct AppState {
    ctx: Mutex<SendableContext>,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let ctx = CustomizationContextBuilder::new()
            .build()
            .map_err(|e| format!("Failed to initialize customization context: {e}"))?;

        Ok(Self {
            ctx: Mutex::new(SendableContext(ctx)),
        })
    }

    /// Returns the current folder icon base, or `None` for vector folder icons.
    pub fn get_folder_icon_base(&self) -> Result<Option<FolderIconBase>, String> {
        let guard = self.ctx.lock().map_err(|e| e.to_string())?;
        Ok(guard.0.folder_icon_base())
    }

    /// Returns the scalable folder icon markup and surface color, if vector.
    pub fn get_folder_icon_svg(&self) -> Result<Option<(String, SurfaceColor)>, String> {
        let guard = self.ctx.lock().map_err(|e| e.to_string())?;
        let surface_color = guard.0.folder_surface_color();
        Ok(guard.0.folder_icon_svg().map(|svg| (svg, surface_color)))
    }
}
