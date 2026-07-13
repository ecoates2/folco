//! Icon types for cross-platform icon representation.
//!
//! This module provides types for representing system icons as a collection
//! of images at various sizes and scales.
//!
//! # Module structure
//!
//! - **Generic types** (`IconImage`, `IconSet`, `RectPx`, `SizePx`, `IconSizeSpec`):
//!   Defined here — usable by any icon workflow (folder or custom).
//! - **Folder-specific types** (`FolderIconBase`, `SurfaceColor`, serializable DTOs):
//!   Defined in the [`folder`] submodule.

mod folder;

use image::RgbaImage;

use crate::error::RenderError;
use crate::layer::ImageSource;

pub use folco_model::{IconSizeSpec, RectPx, SizePx};

pub use folder::{FolderIconBase, SurfaceColor};

// ============================================================================
// IconBase
// ============================================================================

/// The base icon data for an [`IconCustomizer`](crate::IconCustomizer).
///
/// This enum distinguishes between folder icons (which carry surface color
/// metadata for color targeting and decal layers) and user-provided custom
/// images (which have no color metadata).
#[derive(Debug, Clone)]
pub enum IconBase {
    /// System folder icons with surface color metadata.
    ///
    /// Enables all layers: color target, decal, and overlay.
    Folder(FolderIconBase),

    /// User-provided image rasterized to platform sizes.
    ///
    /// Only the overlay layer is applicable; color target and decal
    /// are skipped because there is no surface color reference.
    Custom(IconSet),
}

impl IconBase {
    /// Returns a reference to the underlying icon set.
    pub fn icons(&self) -> &IconSet {
        match self {
            IconBase::Folder(base) => &base.icons,
            IconBase::Custom(icons) => icons,
        }
    }

    /// Returns the surface color, if this is a folder-based icon.
    pub fn surface_color(&self) -> Option<&SurfaceColor> {
        match self {
            IconBase::Folder(base) => Some(&base.surface_color),
            IconBase::Custom(_) => None,
        }
    }

    /// Returns `true` if this is a folder-based icon.
    pub fn is_folder(&self) -> bool {
        matches!(self, IconBase::Folder(_))
    }

    /// Returns `true` if this is a custom (user-provided) icon.
    pub fn is_custom(&self) -> bool {
        matches!(self, IconBase::Custom(_))
    }
}

// ============================================================================
// IconImage
// ============================================================================

/// A single icon image with its associated metadata.
///
/// Icon sets typically contain multiple images at different sizes and scales.
/// For example, macOS uses @1x and @2x variants, Windows uses multiple sizes
/// (16x16, 32x32, 48x48, 256x256), and Linux icon themes have similar patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct IconImage {
    /// The image data in RGBA format.
    pub data: RgbaImage,

    /// The display scale factor.
    ///
    /// - 1.0 for standard resolution (@1x)
    /// - 2.0 for retina/HiDPI (@2x)
    /// - 3.0 for @3x, etc.
    ///
    /// The "logical" size of the icon is `dimensions / scale`.
    pub scale: f32,

    /// The region within the image that contains the actual icon content.
    ///
    /// This is useful for icons that have built-in padding or margins.
    /// If the icon fills the entire image, this will equal
    /// `RectPx::from_size(width, height)`.
    pub content_bounds: RectPx,
}

impl IconImage {
    /// Creates a new icon image with the given data and metadata.
    pub fn new(data: RgbaImage, scale: f32, content_bounds: RectPx) -> Self {
        Self {
            data,
            scale,
            content_bounds,
        }
    }

    /// Creates a new icon image assuming content fills the entire image.
    pub fn new_full_content(data: RgbaImage, scale: f32) -> Self {
        let content_bounds = RectPx::from_size(data.width(), data.height());
        Self::new(data, scale, content_bounds)
    }

    /// Returns the pixel dimensions of the image.
    pub fn dimensions(&self) -> SizePx {
        SizePx::new(self.data.width(), self.data.height())
    }

    /// Returns the logical size of the icon (dimensions / scale).
    ///
    /// For a 64x64 @2x icon, the logical size is 32x32.
    pub fn logical_size(&self) -> (f32, f32) {
        (
            self.data.width() as f32 / self.scale,
            self.data.height() as f32 / self.scale,
        )
    }

    /// Encodes this image's pixels as PNG bytes.
    ///
    /// Shared by the boundary DTOs (Tauri IPC and wasm) that transfer icons
    /// as PNG data rather than raw RGBA buffers.
    pub fn to_png_bytes(&self) -> Result<Vec<u8>, image::ImageError> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        self.data.write_to(&mut cursor, image::ImageFormat::Png)?;
        Ok(cursor.into_inner())
    }
}

// ============================================================================
// IconSet
// ============================================================================

/// A collection of icon images representing a single icon at various sizes and scales.
///
/// System icons typically come as a set of images at different resolutions.
/// This struct groups them together as a cohesive unit.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IconSet {
    /// The individual icon images, typically at various sizes/scales.
    pub images: Vec<IconImage>,
}

impl IconSet {
    /// Creates a new empty icon set.
    pub fn new() -> Self {
        Self { images: Vec::new() }
    }

    /// Creates an icon set from a vector of images.
    pub fn from_images(images: Vec<IconImage>) -> Self {
        Self { images }
    }

    /// Creates an icon set by rendering/resizing an [`ImageSource`] to each spec.
    ///
    /// Each [`IconSizeSpec`] produces one [`IconImage`] with `content_bounds`
    /// set to the full image area (since custom images have no platform-specific
    /// content insets).
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be decoded or rendered at any size.
    pub fn from_image_source(
        source: &ImageSource,
        specs: &[IconSizeSpec],
    ) -> Result<Self, RenderError> {
        let images = specs
            .iter()
            .map(|spec| {
                let rgba = source.render_at_size(spec.max_dimension())?;
                Ok(IconImage::new_full_content(rgba, spec.scale))
            })
            .collect::<Result<Vec<_>, RenderError>>()?;
        Ok(Self::from_images(images))
    }

    /// Adds an image to the icon set.
    pub fn add_image(&mut self, image: IconImage) {
        self.images.push(image);
    }

    /// Returns the number of images in the set.
    pub fn len(&self) -> usize {
        self.images.len()
    }

    /// Returns true if the icon set contains no images.
    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    /// Finds an image by its logical size (closest match).
    ///
    /// This is useful when you need a specific size for display
    /// and want to find the best available variant.
    pub fn find_by_logical_size(&self, target_size: u32) -> Option<&IconImage> {
        self.images.iter().min_by_key(|img| {
            let (logical_w, _) = img.logical_size();
            (logical_w - target_size as f32).abs() as u32
        })
    }

    /// Returns an iterator over the icon images.
    pub fn iter(&self) -> impl Iterator<Item = &IconImage> {
        self.images.iter()
    }
}

impl IntoIterator for IconSet {
    type Item = IconImage;
    type IntoIter = std::vec::IntoIter<IconImage>;

    fn into_iter(self) -> Self::IntoIter {
        self.images.into_iter()
    }
}

impl<'a> IntoIterator for &'a IconSet {
    type Item = &'a IconImage;
    type IntoIter = std::slice::Iter<'a, IconImage>;

    fn into_iter(self) -> Self::IntoIter {
        self.images.iter()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_px_new() {
        let rect = RectPx::new(10, 20, 100, 200);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 200);
        assert_eq!(rect.right(), 110);
        assert_eq!(rect.bottom(), 220);
    }

    #[test]
    fn size_px_is_square() {
        assert!(SizePx::new(100, 100).is_square());
        assert!(!SizePx::new(100, 200).is_square());
    }

    #[test]
    fn icon_image_logical_size() {
        let img = IconImage::new_full_content(RgbaImage::new(64, 64), 2.0);
        let (w, h) = img.logical_size();
        assert_eq!(w, 32.0);
        assert_eq!(h, 32.0);
    }

    #[test]
    fn icon_set_operations() {
        let mut set = IconSet::new();
        assert!(set.is_empty());

        set.add_image(IconImage::new_full_content(RgbaImage::new(16, 16), 1.0));
        set.add_image(IconImage::new_full_content(RgbaImage::new(32, 32), 1.0));

        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());

        // Find closest to 20x20 logical size
        let found = set.find_by_logical_size(20).unwrap();
        // Should find the 16x16 since |16-20| < |32-20|
        assert_eq!(found.dimensions().width, 16);
    }

    #[test]
    fn icon_size_spec_square() {
        let spec = IconSizeSpec::square(64, 2.0);
        assert!(spec.is_square());
        assert_eq!(spec.max_dimension(), 64);
        assert_eq!(spec.scale, 2.0);
    }
}
