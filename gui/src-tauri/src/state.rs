use std::sync::Mutex;

use folco_core::{CustomizationContext, CustomizationContextBuilder, FolderIconBase};

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

    /// Returns the current folder icon base from the context.
    pub fn get_folder_icon_base(&self) -> Result<FolderIconBase, String> {
        let guard = self.ctx.lock().map_err(|e| e.to_string())?;
        Ok(guard.0.folder_icon_base())
    }
}
