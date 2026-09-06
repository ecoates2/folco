//! SVG folder icon base.
//!
//! [`SvgFolderIconBase`] is the vector counterpart to
//! [`FolderIconBase`](super::FolderIconBase): it carries the platform-provided
//! scalable folder icon markup together with the surface color used as a
//! reference for color-based customization.

use folco_model::SurfaceColor;

// ============================================================================
// SvgFolderIconBase
// ============================================================================

/// A scalable folder icon plus surface-color metadata.
///
/// This is the primary input to
/// [`SvgFolderIconCustomizer`](crate::SvgFolderIconCustomizer). It is used when
/// the platform provides a vector folder icon (e.g. GNOME icon themes), in
/// which case raster PNGs are disregarded in favor of the SVG.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgFolderIconBase {
    /// Raw SVG markup for the base folder icon.
    pub svg: String,

    /// The color of the icon's primary content surface.
    ///
    /// Kept for parity with [`FolderIconBase`](super::FolderIconBase) and as a
    /// reference for SVG color layers (e.g. deriving a contrasting decal tint).
    pub surface_color: SurfaceColor,
}

impl SvgFolderIconBase {
    /// Creates a new SVG folder base from markup and a surface color.
    pub fn new(svg: impl Into<String>, surface_color: SurfaceColor) -> Self {
        Self {
            svg: svg.into(),
            surface_color,
        }
    }
}
