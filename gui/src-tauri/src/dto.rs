//! Data-transfer objects for the Tauri IPC boundary.
//!
//! These name the JSON shapes that cross Tauri commands between the Rust
//! backend and the frontend. They are intentionally thin: every type here is a
//! re-export of a type that already owns the canonical serde representation, so
//! there is no duplicated struct definition to keep in sync.
//!
//! - Pure value types come straight from `folco-core`.
//! - The PNG-encoded icon-base types are the shared transfer types from
//!   `folco-transfer`, which the wasm renderer also consumes — so the Tauri
//!   (`serde_json`) and wasm (`serde-wasm-bindgen`) sides are guaranteed to
//!   agree on the wire shape.
//!
//! TODO: generate matching TypeScript for these via tauri-specta.

/// A folder icon base (images + surface color) delivered to the frontend.
pub use folco_transfer::SerializableFolderIconBase as FolderIconBaseDto;

/// A scalable folder icon (markup + surface color) delivered to the frontend.
pub use folco_transfer::SerializableSvgFolderIconBase as SvgFolderIconBaseDto;

/// The set of icon sizes required by the host platform.
pub use folco_core::PlatformSizeSpec as PlatformSizeSpecDto;

/// A folder customization progress event streamed to the frontend.
pub use folco_core::progress::Progress as ProgressDto;

/// The custom image the backend now holds. GUI-only, so defined here.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomImageDto {
    pub revision: u64,
    /// How to decode the bytes.
    pub format: folco_core::ImageFormat,
    /// The pipeline the backend will write it through; the preview must match.
    pub medium: folco_core::OutputMedium,
    pub file_name: String,
}

/// Package metadata from `Cargo.toml` that the Tauri JS API doesn't expose.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppMetadataDto {
    pub description: &'static str,
    /// Each in Cargo's `Name <email>` form.
    pub authors: Vec<&'static str>,
    pub repository: &'static str,
    /// Whether this is a debug (non-release) build.
    pub debug: bool,
}

/// What custom images this platform can write.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CustomImageSupportDto {
    pub write: folco_core::WriteCapabilities,
    /// Lowercase file extensions, without the dot.
    pub extensions: Vec<&'static str>,
}
