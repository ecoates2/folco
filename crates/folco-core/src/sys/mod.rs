//! Platform-specific system icon metadata.
//!
//! This is the only module with platform-conditional code. Each platform file
//! implements the platform-neutral types defined here.
//!
//! Raster folder icons need hand-measured metadata (surface color and content
//! bounds), which only exists for the single stock Windows icon and is exposed
//! as [`RasterFolderMetadata`]. Every other platform must supply a scalable SVG
//! folder icon.
//!
//! Also provides two platform size ladders: the sizes in the system's own
//! folder icon, and the sizes a user-supplied image is rasterized to.

use folco_renderer::{IconSizeSpec, RectPx, SurfaceColor};
use serde::{Deserialize, Serialize};

use crate::image_file::{ImageFile, ImageFormat};

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

// ============================================================================
// WriteCapabilities
// ============================================================================

/// The form a folder icon is written to the system in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMedium {
    /// A set of PNG sizes.
    Raster,
    /// A single scalable SVG.
    Vector,
}

/// Which media this platform's folder icon write path accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct WriteCapabilities {
    pub raster: bool,
    pub vector: bool,
}

impl WriteCapabilities {
    /// What the host platform can write.
    pub fn current_platform() -> Self {
        Self::platform_impl()
    }

    /// Whether `medium` can be written.
    pub fn supports(self, medium: OutputMedium) -> bool {
        match medium {
            OutputMedium::Raster => self.raster,
            OutputMedium::Vector => self.vector,
        }
    }

    /// The medium a custom image of `format` is written in, or `None` if it can't be.
    ///
    /// SVG stays vector where possible and is rasterized otherwise; raster
    /// can't be vectorized, so it needs raster support.
    pub fn custom_image_medium(self, format: ImageFormat) -> Option<OutputMedium> {
        match format {
            ImageFormat::Svg if self.vector => Some(OutputMedium::Vector),
            _ if self.raster => Some(OutputMedium::Raster),
            _ => None,
        }
    }

    /// Extensions of custom image files this platform can write, lowercase.
    pub fn custom_image_extensions(self) -> Vec<&'static str> {
        ImageFile::supported_extensions()
            .filter(|ext| {
                ImageFormat::from_extension(ext)
                    .and_then(|format| self.custom_image_medium(format))
                    .is_some()
            })
            .collect()
    }
}

// ============================================================================
// RasterFolderMetadata
// ============================================================================

/// Hand-measured metadata for a platform's stock raster folder icon.
///
/// Raster customization needs to know where the folder surface sits in each
/// image and what color it is. A platform either has this for its stock icon
/// or must supply a scalable SVG folder icon instead.
#[derive(Debug, Clone, Copy)]
pub struct RasterFolderMetadata {
    /// The color of the icon's primary content surface.
    pub surface_color: SurfaceColor,
    /// Returns the imprintable content region for an image of the given
    /// `(width, height)`.
    pub content_bounds: fn(u32, u32) -> RectPx,
}

impl RasterFolderMetadata {
    /// Returns the metadata for this platform's stock folder icon.
    ///
    /// Returns `None` where the folder icon must be vector (every platform
    /// but Windows).
    pub fn current_platform() -> Option<Self> {
        Self::platform_impl()
    }
}

// ============================================================================
// PlatformSizeSpec
// ============================================================================

/// A platform icon size ladder.
///
/// Two distinct ladders exist per platform, and they need not agree:
///
/// - [`system_folder_icon`](Self::system_folder_icon): the sizes the OS's
///   stock folder icon ships in.
/// - [`custom_image`](Self::custom_image): the sizes a user-supplied image is
///   rasterized to before being written. A custom image has no inherent ladder,
///   so this is driven by what the platform's write path consumes.
///
/// # Example
///
/// ```
/// use folco_core::PlatformSizeSpec;
///
/// let spec = PlatformSizeSpec::custom_image();
/// println!("Custom images are rasterized to {} sizes", spec.sizes().len());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSizeSpec {
    pub(super) sizes: Vec<IconSizeSpec>,
}

impl PlatformSizeSpec {
    /// Creates a new size spec from a list of size specifications.
    pub fn new(sizes: Vec<IconSizeSpec>) -> Self {
        Self { sizes }
    }

    /// Returns the size specs as a slice.
    pub fn sizes(&self) -> &[IconSizeSpec] {
        &self.sizes
    }

    /// The sizes the host OS's stock folder icon ships in.
    ///
    /// - **Windows**: 16, 20, 24, 32, 40, 48, 64, 256 (all @1x)
    /// - **macOS**: 16, 32, 128, 256, 512 (@1x and @2x)
    /// - **Linux**: 16, 22, 24, 32, 48, 64, 128, 256 (all @1x)
    pub fn system_folder_icon() -> Self {
        Self::system_folder_icon_impl()
    }

    /// The sizes a user-supplied image is rasterized to for the host OS.
    ///
    /// - **Windows**, **macOS**: the [system folder icon](Self::system_folder_icon) ladder
    /// - **Linux**: 512 only; the file manager scales the single written file
    pub fn custom_image() -> Self {
        Self::custom_image_impl()
    }
}

#[cfg(test)]
mod write_capabilities_tests {
    use super::*;

    const RASTER_ONLY: WriteCapabilities = WriteCapabilities {
        raster: true,
        vector: false,
    };
    const VECTOR_ONLY: WriteCapabilities = WriteCapabilities {
        raster: false,
        vector: true,
    };

    #[test]
    fn platform_writes_something() {
        let caps = WriteCapabilities::current_platform();
        assert!(caps.raster || caps.vector);
    }

    #[test]
    fn svg_is_rasterized_where_vector_is_unsupported() {
        assert_eq!(
            RASTER_ONLY.custom_image_medium(ImageFormat::Svg),
            Some(OutputMedium::Raster)
        );
    }

    #[test]
    fn raster_is_rejected_where_raster_is_unsupported() {
        assert_eq!(VECTOR_ONLY.custom_image_medium(ImageFormat::Raster), None);
        assert_eq!(VECTOR_ONLY.custom_image_extensions(), vec!["svg"]);
    }
}

#[cfg(test)]
mod platform_size_spec_tests {
    use super::*;

    #[test]
    fn ladders_are_non_empty() {
        assert!(!PlatformSizeSpec::system_folder_icon().sizes().is_empty());
        assert!(!PlatformSizeSpec::custom_image().sizes().is_empty());
    }

    #[test]
    fn all_sizes_are_positive() {
        let specs = [
            PlatformSizeSpec::system_folder_icon(),
            PlatformSizeSpec::custom_image(),
        ];
        for size in specs.iter().flat_map(PlatformSizeSpec::sizes) {
            assert!(size.width > 0);
            assert!(size.height > 0);
            assert!(size.scale > 0.0);
        }
    }

    // The GUI mirrors this shape as a hand-written TypeScript interface.
    #[test]
    fn serializes_as_a_sizes_array() {
        let spec = PlatformSizeSpec::new(vec![IconSizeSpec::square(16, 1.0)]);
        let json = serde_json::to_string(&spec).unwrap();
        assert_eq!(json, r#"{"sizes":[{"width":16,"height":16,"scale":1.0}]}"#);
    }
}
