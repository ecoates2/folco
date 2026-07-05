//! Icon set type conversion between icon-sys and folco-renderer.
//!
//! This module provides utilities for converting between the icon types
//! used by `icon-sys` (system icon operations) and `folco-renderer`
//! (icon customization rendering).
//!
//! The conversion is necessary because:
//! - `icon-sys::IconSet` uses `image::DynamicImage` for flexibility with system APIs
//! - `folco-renderer::IconSet` uses `image::RgbaImage` with additional metadata
//!   (scale factor, content bounds) for rendering operations

use folco_renderer::{IconImage as RendererIconImage, IconSet as RendererIconSet};
use icon_sys::IconSet as SysIconSet;

use crate::sys::get_folder_icon_content_bounds;

/// Converts an `icon-sys` IconSet to a `folco-renderer` IconSet.
///
/// This function is useful for:
/// - Initializing the `FolderIconCustomizer` with system-dumped icons
/// - Providing icons to the WASM version of folco-renderer in folco-gui
///
/// # Arguments
///
/// * `sys_icon_set` - The icon set from icon-sys (typically from `dump_default_folder_icon()`)
///
/// # Returns
///
/// A `folco-renderer` IconSet suitable for use with `FolderIconCustomizer`.
///
/// # Example
///
/// ```ignore
/// use icon_sys::folder_settings::{PlatformDefaultFolderIconProvider, DefaultFolderIconProvider};
/// use folco_core::convert_icon_set;
///
/// let provider = PlatformDefaultFolderIconProvider;
/// let sys_icons = provider.dump_default_folder_icon().unwrap();
/// let renderer_icons = convert_icon_set(&sys_icons);
/// ```
pub fn convert_icon_set(sys_icon_set: &SysIconSet) -> RendererIconSet {
    let images: Vec<RendererIconImage> = sys_icon_set
        .images
        .iter()
        .map(|sys_image| {
            // Convert DynamicImage to RgbaImage
            let rgba = sys_image.data.to_rgba8();

            // Get platform-specific content bounds for this icon size
            let content_bounds = get_folder_icon_content_bounds(rgba.width(), rgba.height());

            // System icons use scale 1.0
            RendererIconImage::new(rgba, 1.0, content_bounds)
        })
        .collect();

    RendererIconSet::from_images(images)
}

/// Converts a `folco-renderer` IconSet back to an `icon-sys` IconSet.
///
/// This is useful when you need to apply rendered icons back to the system.
///
/// # Arguments
///
/// * `renderer_icon_set` - The icon set from folco-renderer (typically after customization)
///
/// # Returns
///
/// An `icon-sys` IconSet suitable for use with `FolderSettingsProvider`.
pub fn convert_icon_set_to_sys(renderer_icon_set: &RendererIconSet) -> SysIconSet {
    let images: Vec<icon_sys::IconImage> = renderer_icon_set
        .iter()
        .map(|renderer_image| {
            // Convert RgbaImage to DynamicImage
            let dynamic = image::DynamicImage::ImageRgba8(renderer_image.data.clone());
            icon_sys::IconImage { data: dynamic }
        })
        .collect();

    // TODO: Support SVG for linux
    SysIconSet { images, svg: None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbaImage};

    #[test]
    fn test_convert_icon_set() {
        // Create a simple sys icon set
        let rgba = RgbaImage::from_pixel(32, 32, image::Rgba([255, 0, 0, 255]));
        let dynamic = DynamicImage::ImageRgba8(rgba);
        let sys_set = SysIconSet {
            images: vec![icon_sys::IconImage { data: dynamic }],
            svg: None,
        };

        // Convert to renderer icon set
        let renderer_set = convert_icon_set(&sys_set);

        assert_eq!(renderer_set.len(), 1);
        let img = renderer_set.iter().next().unwrap();
        assert_eq!(img.dimensions().width, 32);
        assert_eq!(img.dimensions().height, 32);
        assert_eq!(img.scale, 1.0);
    }

    #[test]
    fn test_convert_icon_set_to_sys() {
        // Create a simple renderer icon set
        let rgba = RgbaImage::from_pixel(16, 16, image::Rgba([0, 255, 0, 255]));
        let renderer_img = RendererIconImage::new_full_content(rgba, 1.0);
        let renderer_set = RendererIconSet::from_images(vec![renderer_img]);

        // Convert to sys icon set
        let sys_set = convert_icon_set_to_sys(&renderer_set);

        assert_eq!(sys_set.images.len(), 1);
        let img = &sys_set.images[0];
        assert_eq!(img.data.width(), 16);
        assert_eq!(img.data.height(), 16);
    }

    #[test]
    fn test_roundtrip_conversion() {
        // Create original sys icon set
        let rgba = RgbaImage::from_pixel(48, 48, image::Rgba([0, 0, 255, 255]));
        let dynamic = DynamicImage::ImageRgba8(rgba);
        let original_sys_set = SysIconSet {
            images: vec![icon_sys::IconImage { data: dynamic }],
            svg: None,
        };

        // Convert to renderer and back
        let renderer_set = convert_icon_set(&original_sys_set);
        let roundtrip_sys_set = convert_icon_set_to_sys(&renderer_set);

        // Verify dimensions preserved
        assert_eq!(
            original_sys_set.images[0].data.width(),
            roundtrip_sys_set.images[0].data.width()
        );
        assert_eq!(
            original_sys_set.images[0].data.height(),
            roundtrip_sys_set.images[0].data.height()
        );
    }
}
