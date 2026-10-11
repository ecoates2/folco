//! Conversion from an `icon-sys` icon set to a [`SvgFolderIconBase`].
//!
//! Used on platforms without raster folder icon metadata (see
//! [`RasterFolderMetadata`](crate::sys::RasterFolderMetadata)), where the
//! folder icon theme must supply a scalable SVG variant. Raster images on the
//! set are disregarded.

use crate::error::{Error, Result};

use folco_renderer::SvgFolderIconBase;
use icon_sys::IconSet as SysIconSet;

/// Builds the vector folder base from the set's SVG variant.
///
/// # Errors
///
/// Returns [`Error::MissingScalableFolderIcon`] if the set has no SVG variant.
pub(crate) fn from_sys_icon_set(sys_icon_set: SysIconSet) -> Result<SvgFolderIconBase> {
    sys_icon_set
        .svg
        .map(SvgFolderIconBase::new)
        .ok_or(Error::MissingScalableFolderIcon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_svg_variant() {
        let sys_set = SysIconSet {
            images: Vec::new(),
            svg: Some("<svg/>".to_string()),
        };
        let base = from_sys_icon_set(sys_set).unwrap();
        assert_eq!(base.svg, "<svg/>");
    }

    #[test]
    fn rejects_raster_only_set() {
        let sys_set = SysIconSet {
            images: Vec::new(),
            svg: None,
        };
        assert!(matches!(
            from_sys_icon_set(sys_set),
            Err(Error::MissingScalableFolderIcon)
        ));
    }
}
