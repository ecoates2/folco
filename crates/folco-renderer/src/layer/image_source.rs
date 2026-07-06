//! Image source types for renderable image data — vector or raster.
//!
//! [`ImageSource`] is a superset of [`SvgSource`] that can also carry
//! pre-rasterized image data (PNG-encoded). It is used by layers that
//! can accept either vector or raster input (e.g., overlays, custom icons).

use image::RgbaImage;
use serde::{Deserialize, Serialize};

use super::svg::{SvgSource, render_source};
use crate::error::RenderError;

// ============================================================================
// ImageSource
// ============================================================================

/// A source for renderable image data — vector or raster.
///
/// This enum generalizes [`SvgSource`] to also support pre-rasterized images,
/// enabling layers and customizers to accept user-provided bitmaps alongside
/// SVG/emoji sources.
///
/// # Variants
///
/// - **Svg**: Wraps an [`SvgSource`] (raw SVG markup, emoji character, or emoji name).
/// - **Raster**: PNG-encoded image bytes. PNG is used because it is lossless and
///   `Serialize`/`Deserialize`-friendly for profiles, IPC, and WASM transfer.
///
/// # Example
///
/// ```
/// use folco_renderer::layer::ImageSource;
/// use folco_renderer::SvgSource;
///
/// // From raw SVG
/// let svg_source = ImageSource::from(SvgSource::from_svg("<svg>...</svg>"));
///
/// // From PNG bytes
/// let png_bytes: Vec<u8> = std::fs::read("icon.png").unwrap_or_default();
/// let raster_source = ImageSource::raster(png_bytes);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ImageSource {
    /// Scalable vector source (SVG / emoji).
    Svg(SvgSource),

    /// PNG-encoded raster image bytes.
    ///
    /// At render time these are decoded and resized (Lanczos3) to the
    /// requested target dimensions.
    Raster(Vec<u8>),
}

impl ImageSource {
    /// Creates a raster image source from PNG-encoded bytes.
    pub fn raster(png_data: Vec<u8>) -> Self {
        Self::Raster(png_data)
    }

    /// Creates an SVG image source from raw SVG markup.
    pub fn svg(svg: impl Into<String>) -> Self {
        Self::Svg(SvgSource::from_svg(svg))
    }

    /// Creates an image source from an `image::DynamicImage` by encoding to PNG.
    ///
    /// # Errors
    ///
    /// Returns an error if PNG encoding fails.
    pub fn from_dynamic_image(img: &image::DynamicImage) -> Result<Self, RenderError> {
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png)
            .map_err(|e| RenderError::ImageEncode { source: e })?;
        Ok(Self::Raster(buf.into_inner()))
    }

    /// Creates an image source from an `RgbaImage` by encoding to PNG.
    ///
    /// # Errors
    ///
    /// Returns an error if PNG encoding fails.
    pub fn from_rgba_image(img: &RgbaImage) -> Result<Self, RenderError> {
        Self::from_dynamic_image(&image::DynamicImage::ImageRgba8(img.clone()))
    }

    /// Returns `true` if this is an SVG-based source.
    pub fn is_svg(&self) -> bool {
        matches!(self, Self::Svg(_))
    }

    /// Returns `true` if this is a raster image source.
    pub fn is_raster(&self) -> bool {
        matches!(self, Self::Raster(_))
    }

    /// Renders this source to an RGBA image at the specified size.
    ///
    /// - **SVG sources** are rendered via resvg/usvg, fitting into `size × size`.
    /// - **Raster sources** are decoded from PNG and resized using Lanczos3
    ///   filtering to fit within `size × size` while preserving aspect ratio.
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be decoded or rendered.
    pub fn render_at_size(&self, size: u32) -> Result<RgbaImage, RenderError> {
        match self {
            Self::Svg(source) => render_source(source, size),
            Self::Raster(png_data) => {
                let img = image::load_from_memory(png_data).map_err(RenderError::ImageDecode)?;
                let rgba = img.to_rgba8();

                // If already the right size, return as-is
                let max_dim = rgba.width().max(rgba.height());
                if max_dim == size {
                    return Ok(rgba);
                }

                // Resize preserving aspect ratio (fit within size×size)
                let resized = img.resize(size, size, image::imageops::FilterType::Lanczos3);
                Ok(resized.to_rgba8())
            }
        }
    }

    /// Renders this source to an RGBA image, optionally replacing all colors.
    ///
    /// Color replacement only applies to SVG sources. For raster sources,
    /// the color parameter is ignored and the image is returned as-is.
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be decoded or rendered.
    pub fn render_at_size_with_color(
        &self,
        size: u32,
        fill_color: Option<(u8, u8, u8, u8)>,
    ) -> Result<RgbaImage, RenderError> {
        match self {
            Self::Svg(source) => {
                use super::svg::render_source_with_color;
                render_source_with_color(source, size, fill_color)
            }
            Self::Raster(_) => {
                // Color replacement doesn't apply to raster images
                self.render_at_size(size)
            }
        }
    }
}

impl From<SvgSource> for ImageSource {
    fn from(source: SvgSource) -> Self {
        Self::Svg(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_source_from_svg_source() {
        let svg = SvgSource::from_svg("<svg></svg>");
        let source = ImageSource::from(svg);
        assert!(source.is_svg());
        assert!(!source.is_raster());
    }

    #[test]
    fn image_source_raster_constructor() {
        let source = ImageSource::raster(vec![1, 2, 3]);
        assert!(source.is_raster());
        assert!(!source.is_svg());
    }

    #[test]
    fn image_source_svg_constructor() {
        let source = ImageSource::svg("<svg></svg>");
        assert!(source.is_svg());
    }

    #[test]
    fn image_source_from_rgba_image() {
        let img = RgbaImage::new(16, 16);
        let source = ImageSource::from_rgba_image(&img).unwrap();
        assert!(source.is_raster());

        // Should be able to render back
        let rendered = source.render_at_size(16).unwrap();
        assert_eq!(rendered.width(), 16);
        assert_eq!(rendered.height(), 16);
    }

    #[test]
    fn image_source_serialization_roundtrip() {
        let source = ImageSource::svg("<svg></svg>");
        let json = serde_json::to_string(&source).unwrap();
        let restored: ImageSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, restored);
    }
}
