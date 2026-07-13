//! Folder-specific icon types.
//!
//! [`FolderIconBase`] pairs an [`IconSet`] with the surface color needed for
//! color targeting. The serializable transfer DTOs now live in the boundary
//! crates that own them (`folco-renderer-wasm` for the wasm boundary and the
//! Tauri app for IPC), each converting from `FolderIconBase` directly.

use super::IconSet;

pub use folco_model::SurfaceColor;

// ============================================================================
// FolderIconBase
// ============================================================================

/// A base icon set combined with metadata about the icon's appearance.
///
/// This is the primary input to [`FolderIconCustomizer`](crate::FolderIconCustomizer),
/// pairing the icon images with the surface color needed to compute
/// color target deltas from target colors.
#[derive(Debug, Clone, PartialEq)]
pub struct FolderIconBase {
    /// The base icon images at various sizes.
    pub icons: IconSet,
    /// The HSL color of the icon's primary content surface.
    pub surface_color: SurfaceColor,
}

impl FolderIconBase {
    /// Creates a new icon base with the given icons and surface color.
    pub fn new(icons: IconSet, surface_color: SurfaceColor) -> Self {
        Self {
            icons,
            surface_color,
        }
    }
}
