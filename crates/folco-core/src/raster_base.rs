//! Conversion from an `icon-sys` icon set to a raster [`FolderIconBase`].
//!
//! Raster folder bases need hand-measured metadata (surface color and per-size
//! content bounds) for the platform's stock folder icon. That metadata, and
//! whether a platform has any, lives in [`sys`](crate::sys); this module only
//! applies it.
//!
//! The conversion is necessary because `icon-sys` stores images as
//! `image::DynamicImage` for flexibility with system APIs, while
//! `folco-renderer` uses `image::RgbaImage` with additional metadata (scale
//! factor, content bounds) for rendering.

use crate::sys::RasterFolderMetadata;

use folco_renderer::{FolderIconBase, IconImage as RendererIconImage, IconSet as RendererIconSet};
use icon_sys::IconSet as SysIconSet;

/// Builds the raster folder base from the platform's stock folder icon.
///
/// Any SVG variant on the set is disregarded.
pub(crate) fn from_sys_icon_set(
    sys_icon_set: SysIconSet,
    metadata: &RasterFolderMetadata,
) -> FolderIconBase {
    FolderIconBase::new(
        into_renderer_icon_set(sys_icon_set, metadata),
        metadata.surface_color,
    )
}

/// Converts an `icon-sys` IconSet to a `folco-renderer` IconSet.
///
/// Content bounds are taken from the platform's stock folder icon metadata, so
/// the input is expected to be that icon (typically from
/// `dump_default_folder_icon()`).
///
/// This function is useful for:
/// - Initializing the `FolderIconCustomizer` with system-dumped icons
/// - Providing icons to the WASM version of folco-renderer in folco-gui
///
/// # Panics
///
/// Panics on platforms without raster folder icon metadata (everything but
/// Windows), or if an image's size has no known content bounds.
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
    let metadata = platform_metadata();
    let images = sys_icon_set
        .images
        .iter()
        .map(|sys_image| to_renderer_image(sys_image.data.to_rgba8(), &metadata))
        .collect();

    RendererIconSet::from_images(images)
}

/// Consuming [`convert_icon_set`] with explicit metadata: pixel buffers that
/// are already RGBA8 are reused.
pub(crate) fn into_renderer_icon_set(
    sys_icon_set: SysIconSet,
    metadata: &RasterFolderMetadata,
) -> RendererIconSet {
    let images = sys_icon_set
        .images
        .into_iter()
        .map(|sys_image| to_renderer_image(sys_image.data.into_rgba8(), metadata))
        .collect();

    RendererIconSet::from_images(images)
}

/// Returns this platform's raster folder metadata.
///
/// # Panics
///
/// Panics on platforms whose folder icon must be vector.
pub(crate) fn platform_metadata() -> RasterFolderMetadata {
    RasterFolderMetadata::current_platform()
        .expect("this platform has no raster folder icon metadata; its folder icon must be SVG")
}

fn to_renderer_image(rgba: image::RgbaImage, metadata: &RasterFolderMetadata) -> RendererIconImage {
    let content_bounds = (metadata.content_bounds)(rgba.width(), rgba.height());

    // System icons use scale 1.0
    RendererIconImage::new(rgba, 1.0, content_bounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::SystemFormat;
    use folco_renderer::{RectPx, SurfaceColor};
    use image::RgbaImage;

    const TEST_METADATA: RasterFolderMetadata = RasterFolderMetadata {
        surface_color: SurfaceColor::new(10, 20, 30),
        content_bounds: |width, height| RectPx::new(1, 2, width - 2, height - 4),
    };

    fn sys_set_of(size: u32) -> SysIconSet {
        let rgba = RgbaImage::from_pixel(size, size, image::Rgba([255, 0, 0, 255]));
        SysIconSet {
            images: vec![icon_sys::IconImage {
                data: image::DynamicImage::ImageRgba8(rgba),
            }],
            svg: None,
        }
    }

    #[test]
    fn test_into_renderer_icon_set() {
        let renderer_set = into_renderer_icon_set(sys_set_of(32), &TEST_METADATA);

        assert_eq!(renderer_set.len(), 1);
        let img = renderer_set.iter().next().unwrap();
        assert_eq!(img.dimensions().width, 32);
        assert_eq!(img.dimensions().height, 32);
        assert_eq!(img.scale, 1.0);
        assert_eq!(img.content_bounds, RectPx::new(1, 2, 30, 28));
    }

    #[test]
    fn test_roundtrip_conversion() {
        let renderer_set = into_renderer_icon_set(sys_set_of(48), &TEST_METADATA);
        let roundtrip_sys_set = renderer_set.into_sys();

        assert_eq!(roundtrip_sys_set.images[0].data.width(), 48);
        assert_eq!(roundtrip_sys_set.images[0].data.height(), 48);
    }

    #[test]
    fn test_from_sys_icon_set_uses_metadata_surface_color() {
        let base = from_sys_icon_set(sys_set_of(16), &TEST_METADATA);
        assert_eq!(base.surface_color, TEST_METADATA.surface_color);
        assert_eq!(base.icons.len(), 1);
    }
}
