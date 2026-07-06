//! Folder-specific icon types and serializable transfer types.
//!
//! These types pair an [`IconSet`] with metadata specific to folder icon
//! customization (surface color, content bounds) and provide serializable
//! DTOs for IPC/WASM transfer.

use std::io::Cursor;

use image::ImageFormat;
use serde::{Deserialize, Serialize};

use super::{IconImage, IconSet, RectPx};

// ============================================================================
// SurfaceColor
// ============================================================================

/// The RGB color of an icon's primary content surface.
///
/// Used as the reference point when computing color target deltas.
/// Each platform defines its own surface color (e.g., RGB(255, 217, 112)
/// for the golden-yellow Windows folder icon).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct SurfaceColor {
    /// Red channel (0–255).
    pub r: u8,
    /// Green channel (0–255).
    pub g: u8,
    /// Blue channel (0–255).
    pub b: u8,
}

impl SurfaceColor {
    /// Creates a new surface color from RGB values.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

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

// ============================================================================
// Serializable transfer types
// ============================================================================

/// PNG-encoded representation of an [`IconImage`] for serialization and IPC.
///
/// Unlike [`IconImage`] (which holds `RgbaImage` in memory), this type stores
/// the image as PNG bytes, making it suitable for JSON serialization,
/// Tauri IPC, and wasm-bindgen transfer.
///
/// Use `TryFrom<&IconImage>` to convert, or [`SerializableFolderIconBase::try_from`]
/// to convert an entire [`FolderIconBase`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct SerializableIconImage {
    /// PNG-encoded image bytes.
    pub png_data: Vec<u8>,
    /// Display scale factor (1.0 for @1x, 2.0 for @2x, etc.)
    pub scale: f32,
    /// Pixel width of the image.
    pub width: u32,
    /// Pixel height of the image.
    pub height: u32,
    /// The region within the image containing the actual icon content.
    ///
    /// For icons with built-in padding/margins this is smaller than the
    /// full image dimensions. If absent during deserialization, defaults
    /// to the full image size for backwards compatibility.
    #[serde(default)]
    pub content_bounds: Option<RectPx>,
}

/// Serializable representation of an [`FolderIconBase`] for IPC transfer.
///
/// Contains everything needed to reconstruct an [`FolderIconBase`] on the
/// receiving end (e.g., in a WASM `CanvasRenderer`).
///
/// Use `TryFrom<&FolderIconBase>` to convert from an in-memory [`FolderIconBase`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify_next::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct SerializableFolderIconBase {
    /// PNG-encoded icon images at various sizes/scales.
    pub images: Vec<SerializableIconImage>,
    /// The surface color of the base icon.
    pub surface_color: SurfaceColor,
}

impl TryFrom<&IconImage> for SerializableIconImage {
    type Error = image::ImageError;

    fn try_from(img: &IconImage) -> std::result::Result<Self, Self::Error> {
        let mut png_bytes = Cursor::new(Vec::new());
        img.data.write_to(&mut png_bytes, ImageFormat::Png)?;

        Ok(Self {
            png_data: png_bytes.into_inner(),
            scale: img.scale,
            width: img.data.width(),
            height: img.data.height(),
            content_bounds: Some(img.content_bounds),
        })
    }
}

impl TryFrom<&FolderIconBase> for SerializableFolderIconBase {
    type Error = image::ImageError;

    fn try_from(base: &FolderIconBase) -> std::result::Result<Self, Self::Error> {
        let images = base
            .icons
            .iter()
            .map(SerializableIconImage::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(Self {
            images,
            surface_color: base.surface_color,
        })
    }
}

impl SerializableFolderIconBase {
    /// Decodes the PNG images and reconstructs an [`FolderIconBase`].
    ///
    /// This is the inverse of `TryFrom<&FolderIconBase>`.
    pub fn into_folder_icon_base(self) -> Result<FolderIconBase, image::ImageError> {
        let mut icon_set = IconSet::new();

        for img in &self.images {
            let rgba = image::load_from_memory(&img.png_data)?.to_rgba8();
            let width = rgba.width();
            let height = rgba.height();
            let bounds = img
                .content_bounds
                .unwrap_or(RectPx::from_size(width, height));
            icon_set.add_image(IconImage::new(rgba, img.scale, bounds));
        }

        Ok(FolderIconBase::new(icon_set, self.surface_color))
    }
}
