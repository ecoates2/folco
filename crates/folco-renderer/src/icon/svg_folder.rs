//! SVG folder icon base.
//!
//! [`SvgFolderIconBase`] is the vector counterpart to
//! [`FolderIconBase`](super::FolderIconBase): it carries the platform-provided
//! scalable folder icon markup.

// ============================================================================
// SvgFolderIconBase
// ============================================================================

/// A scalable folder icon.
///
/// This is the primary input to
/// [`SvgFolderIconCustomizer`](crate::SvgFolderIconCustomizer). It is used when
/// the platform provides a vector folder icon (e.g. GNOME icon themes), in
/// which case raster PNGs are disregarded in favor of the SVG.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgFolderIconBase {
    /// Raw SVG markup for the base folder icon.
    pub svg: String,
}

impl SvgFolderIconBase {
    /// Creates a new SVG folder base from markup.
    pub fn new(svg: impl Into<String>) -> Self {
        Self { svg: svg.into() }
    }
}
