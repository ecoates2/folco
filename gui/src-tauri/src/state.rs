use std::path::{Path, PathBuf};
use std::sync::Mutex;

use folco_core::progress::ProgressSender;
use folco_core::{
    CustomizationContext, CustomizationContextBuilder, CustomizationProfile, ImageFile,
    ImageSource,
};

use crate::dto::{CustomImageDto, FolderIconBaseDto, ImageKindDto};

/// Wrapper to make `CustomizationContext` usable in Tauri managed state.
///
/// `CustomizationContext` contains platform COM pointers (e.g., `IKnownFolderManager`
/// on Windows) that are not `Send`/`Sync`. Since we always access it behind a `Mutex`,
/// this is safe.
struct SendableContext(CustomizationContext);

// SAFETY: `CustomizationContext` holds platform-specific handles (e.g., Windows COM pointers)
/// that are not `Send`/`Sync`. This wrapper is safe because:
///
/// 1. **Send**: Tauri may send `AppState` across thread boundaries (e.g., from the main thread
///    to a command handler). We guarantee mutual exclusion by requiring `Mutex::lock()` before
///    any access, so no two threads ever touch the inner `CustomizationContext` concurrently.
/// 2. **Sync**: `Mutex` itself is `Sync`, but the inner type's lack of `Sync` prevents the
///    compiler from auto-deriving it. We manually implement `Sync` because all public access
///    to the context goes through `AppState::with_ctx()`, which acquires the lock first.
///    The invariant — "every access is serialized through the mutex" — is enforced by the
///    module's API surface: `SendableContext` is `pub(crate)` with no public methods, and
///    the only way to read it is via `with_ctx()` which borrows behind the lock.
unsafe impl Send for SendableContext {}
unsafe impl Sync for SendableContext {}

/// The user's custom base image, as last loaded.
///
/// The backend owns the image; the frontend previews a copy of these exact bytes
/// and quotes `revision` back, so an operation can't write a different image than
/// the one on screen.
#[derive(Default)]
struct CustomImageSlot {
    revision: u64,
    file: Option<ImageFile>,
}

impl CustomImageSlot {
    fn get(&self, revision: u64) -> Result<&ImageFile, String> {
        match &self.file {
            Some(file) if revision == self.revision => Ok(file),
            _ => Err("The custom image changed since it was previewed; pick it again.".into()),
        }
    }
}

/// Tauri managed state wrapping the `CustomizationContext`.
///
/// The context is built on first use rather than at startup: building it can hit
/// the disk (icon cache) or extract the platform folder icon, and doing that
/// eagerly delays window creation by that whole cost.
pub struct AppState {
    ctx: Mutex<Option<SendableContext>>,
    custom_image: Mutex<CustomImageSlot>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ctx: Mutex::new(None),
            custom_image: Mutex::default(),
        }
    }

    /// Runs `f` against the customization context, building it if this is the first access.
    fn with_ctx<T>(&self, f: impl FnOnce(&CustomizationContext) -> T) -> Result<T, String> {
        self.with_ctx_mut(|ctx| f(ctx))
    }

    /// Mutable counterpart of [`with_ctx`](Self::with_ctx).
    fn with_ctx_mut<T>(
        &self,
        f: impl FnOnce(&mut CustomizationContext) -> T,
    ) -> Result<T, String> {
        let mut guard = self.ctx.lock().map_err(|e| e.to_string())?;

        if guard.is_none() {
            let ctx = CustomizationContextBuilder::new()
                .build()
                .map_err(|e| format!("Failed to initialize customization context: {e}"))?;
            *guard = Some(SendableContext(ctx));
        }

        match guard.as_mut() {
            Some(ctx) => Ok(f(&mut ctx.0)),
            None => Err("customization context missing after initialization".to_string()),
        }
    }

    /// Builds the context ahead of the first command that needs it.
    ///
    /// Callers that arrive mid-build block on the `Mutex` rather than duplicating work.
    pub fn warm(&self) -> Result<(), String> {
        self.with_ctx(|_| ())
    }

    /// Returns the current folder icon base as a DTO, or `None` for vector folder icons.
    ///
    /// Encodes while holding the lock so the base icons are borrowed, not cloned.
    pub fn get_folder_icon_base(&self) -> Result<Option<FolderIconBaseDto>, String> {
        self.with_ctx(|ctx| {
            ctx.folder_icon_base()
                .map(FolderIconBaseDto::try_from)
                .transpose()
                .map_err(|e| e.to_string())
        })?
    }

    /// Returns the scalable folder icon markup, if vector.
    pub fn get_folder_icon_svg(&self) -> Result<Option<String>, String> {
        self.with_ctx(|ctx| ctx.folder_icon_svg())
    }

    /// Applies `profile` to the context and writes the result to every folder.
    ///
    /// Blocks the calling thread for the whole operation; per-folder outcomes are
    /// reported through `progress`, so `Ok` does not mean every folder succeeded.
    pub fn customize_folders(
        &self,
        folders: Vec<PathBuf>,
        profile: CustomizationProfile,
        progress: ProgressSender,
    ) -> Result<(), String> {
        self.with_ctx_mut(|ctx| {
            // Applying anyway would write a stock-looking icon and report success.
            let unsupported = ctx.unsupported_layers(&profile);
            if !unsupported.is_empty() {
                let details = unsupported
                    .iter()
                    .map(|entry| format!("{} ({})", entry.layer, entry.reason))
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!("This system can't apply: {details}"));
            }

            tauri::async_runtime::block_on(ctx.customize_folders_async(folders, profile, progress));
            Ok(())
        })?
    }

    /// Restores the default icon on every folder, blocking like
    /// [`customize_folders`](Self::customize_folders).
    pub fn reset_folders(
        &self,
        folders: Vec<PathBuf>,
        progress: ProgressSender,
    ) -> Result<(), String> {
        self.with_ctx(|ctx| {
            tauri::async_runtime::block_on(ctx.reset_folders_async(folders, progress));
        })
    }

    /// Reads `path` and makes it the custom image, superseding any earlier one.
    pub fn load_custom_image(&self, path: &Path) -> Result<CustomImageDto, String> {
        let file = ImageFile::read(path).map_err(|e| e.to_string())?;
        let kind = match file {
            ImageFile::Svg(_) => ImageKindDto::Svg,
            ImageFile::Raster(_) => ImageKindDto::Raster,
        };
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        let mut slot = self.custom_image.lock().map_err(|e| e.to_string())?;
        slot.revision += 1;
        slot.file = Some(file);

        Ok(CustomImageDto {
            revision: slot.revision,
            kind,
            file_name,
        })
    }

    /// The contents of custom image `revision`, for the frontend's preview.
    pub fn custom_image_bytes(&self, revision: u64) -> Result<Vec<u8>, String> {
        let slot = self.custom_image.lock().map_err(|e| e.to_string())?;
        Ok(slot.get(revision)?.bytes().to_vec())
    }

    /// Writes custom image `revision`, decorated per `profile`, to every folder.
    ///
    /// Blocks like [`customize_folders`](Self::customize_folders).
    pub fn customize_folders_with_image(
        &self,
        folders: Vec<PathBuf>,
        revision: u64,
        profile: CustomizationProfile,
        progress: ProgressSender,
    ) -> Result<(), String> {
        // Cloned so the slot isn't held for the whole write.
        let source: ImageSource = {
            let slot = self.custom_image.lock().map_err(|e| e.to_string())?;
            slot.get(revision)?.clone().into()
        };

        self.with_ctx(|ctx| {
            tauri::async_runtime::block_on(
                ctx.customize_folders_with_image_async(folders, &source, profile, progress),
            )
            .map_err(|e| e.to_string())
        })?
    }
}
