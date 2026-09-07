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
///
/// The context is built on first use rather than at startup: building it can hit
/// the disk (icon cache) or extract the platform folder icon, and doing that
/// eagerly delays window creation by that whole cost.
pub struct AppState {
    ctx: Mutex<Option<SendableContext>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ctx: Mutex::new(None),
        }
    }

    /// Runs `f` against the customization context, building it if this is the first access.
    fn with_ctx<T>(&self, f: impl FnOnce(&CustomizationContext) -> T) -> Result<T, String> {
        let mut guard = self.ctx.lock().map_err(|e| e.to_string())?;

        if guard.is_none() {
            let ctx = CustomizationContextBuilder::new()
                .build()
                .map_err(|e| format!("Failed to initialize customization context: {e}"))?;
            *guard = Some(SendableContext(ctx));
        }

        match guard.as_ref() {
            Some(ctx) => Ok(f(&ctx.0)),
            None => Err("customization context missing after initialization".to_string()),
        }
    }

    /// Returns the current folder icon base, or `None` for vector folder icons.
    pub fn get_folder_icon_base(&self) -> Result<Option<FolderIconBase>, String> {
        self.with_ctx(|ctx| ctx.folder_icon_base())
    }

    /// Returns the scalable folder icon markup and surface color, if vector.
    pub fn get_folder_icon_svg(&self) -> Result<Option<(String, SurfaceColor)>, String> {
        self.with_ctx(|ctx| {
            let surface_color = ctx.folder_surface_color();
            ctx.folder_icon_svg().map(|svg| (svg, surface_color))
        })
    }
}
