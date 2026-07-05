//! Windows-specific system icon metadata.

use folco_renderer::{RectPx, SurfaceColor};
use icon_sys::icon::sys::windows::WindowsIconSize;

/// The default Windows folder icon surface color: RGB(255, 217, 112).
///
/// This is the golden-yellow color of the standard Windows folder icon,
/// used as the reference point for computing color target deltas.
pub const SURFACE_COLOR: SurfaceColor = SurfaceColor::new(255, 217, 112);

/// Returns the content bounds for a Windows system folder icon.
///
/// Windows folder icons from shell32.dll have specific content regions
/// that don't fill the entire icon canvas. These bounds define the
/// "imprintable surface" where the actual folder visual exists.
///
/// # Arguments
///
/// * `dimension` - The icon width in pixels (must be a valid Windows icon size)
/// * `_height` - The icon height (unused, accepted for API consistency)
///
/// # Returns
///
/// A `RectPx` describing the region containing the actual icon content.
///
/// # Panics
///
/// Panics if `dimension` is not a valid Windows icon size (16, 20, 24, 32, 40, 48, 64, or 256).
pub fn get_folder_icon_content_bounds(dimension: u32, _height: u32) -> RectPx {
    let size = WindowsIconSize::from_dimension(dimension)
        .expect("Invalid Windows icon dimension");

    match size {
        WindowsIconSize::Px16 => RectPx::new(0, 4, 16, 9),
        WindowsIconSize::Px20 => RectPx::new(1, 6, 18, 10),
        WindowsIconSize::Px24 => RectPx::new(1, 6, 22, 13),
        WindowsIconSize::Px32 => RectPx::new(2, 8, 28, 17),
        WindowsIconSize::Px40 => RectPx::new(2, 10, 38, 22),
        WindowsIconSize::Px48 => RectPx::new(3, 11, 42, 27),
        WindowsIconSize::Px64 => RectPx::new(4, 16, 56, 36),
        WindowsIconSize::Px256 => RectPx::new(16, 62, 224, 144),
    }
}

use folco_renderer::IconSizeSpec;
use super::PlatformSizeSpec;

impl PlatformSizeSpec {
    pub(super) fn platform_impl() -> Self {
        Self {
            sizes: vec![
                IconSizeSpec::square(16, 1.0),
                IconSizeSpec::square(20, 1.0),
                IconSizeSpec::square(24, 1.0),
                IconSizeSpec::square(32, 1.0),
                IconSizeSpec::square(40, 1.0),
                IconSizeSpec::square(48, 1.0),
                IconSizeSpec::square(64, 1.0),
                IconSizeSpec::square(256, 1.0),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_bounds_16() {
        let bounds = get_folder_icon_content_bounds(16, 16);
        assert_eq!(bounds.x, 0);
        assert_eq!(bounds.y, 4);
        assert_eq!(bounds.width, 16);
        assert_eq!(bounds.height, 9);
    }

    #[test]
    fn test_content_bounds_32() {
        let bounds = get_folder_icon_content_bounds(32, 32);
        assert_eq!(bounds.x, 2);
        assert_eq!(bounds.y, 8);
        assert_eq!(bounds.width, 28);
        assert_eq!(bounds.height, 17);
    }

    #[test]
    fn test_content_bounds_256() {
        let bounds = get_folder_icon_content_bounds(256, 256);
        assert_eq!(bounds.x, 16);
        assert_eq!(bounds.y, 62);
        assert_eq!(bounds.width, 224);
        assert_eq!(bounds.height, 144);
    }

    #[test]
    fn test_all_sizes_valid() {
        for size in WindowsIconSize::all() {
            let dim = size.dimension();
            let bounds = get_folder_icon_content_bounds(dim, dim);
            // Content bounds should be within the icon
            assert!(bounds.x + bounds.width <= dim);
            assert!(bounds.y + bounds.height <= dim);
        }
    }
}
