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

/// How the frontend should interpret custom image bytes.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageKindDto {
    Svg,
    Raster,
}

/// The custom image the backend now holds. GUI-only, so defined here.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomImageDto {
    pub revision: u64,
    pub kind: ImageKindDto,
    pub file_name: String,
}
