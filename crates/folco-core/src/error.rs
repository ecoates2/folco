//! Error types for folco-core.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during folco-core operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to get or create app data directory.
    #[error("failed to get app data directory: {0}")]
    AppDataDir(String),

    /// Error from icon-sys crate.
    #[error("icon system error: {0}")]
    IconSys(#[from] icon_sys::Error),

    /// Error during icon caching.
    #[error("cache error: {0}")]
    Cache(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error during folder customization.
    #[error("failed to customize folder '{0}': {1}")]
    FolderCustomization(PathBuf, String),

    /// Error during folder reset.
    #[error("failed to reset folder '{0}': {1}")]
    FolderReset(PathBuf, String),

    /// Image processing error.
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),

    /// Context not properly initialized.
    #[error("context not initialized: {0}")]
    NotInitialized(String),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Folder settings error from icon-sys.
    #[error("folder settings error: {0}")]
    FolderSettings(#[from] icon_sys::folder_settings::FolderSettingsError),

    /// Icon rendering error from folco-renderer.
    #[error("rendering error: {0}")]
    Render(#[from] folco_renderer::RenderError),
}
