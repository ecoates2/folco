//! Serializable wasm DTOs for transferring folder icon bases to JavaScript.
//!
//! These are the WASM boundary's own transfer types (PNG-encoded), derived
//! with `tsify` in **`js`** mode so they cross the wasm-bindgen ABI as native
//! JS objects (via `serde-wasm-bindgen`) rather than JSON strings.
//!
//! IMPORTANT — cross-boundary shape constraint:
//! The Folco Tauri backend produces the *same JSON shape* for these (see the
//! app's IPC DTOs in `gui/src-tauri/src/dto.rs`), and the frontend pipes the
//! Tauri command result straight into
//! [`CanvasRenderer::from_folder_icon_base`](crate::CanvasRenderer). Because
//! Tauri serializes with `serde_json` while this crate deserializes with
//! `serde-wasm-bindgen`, any field added here must serialize identically under
//! both. Avoid `HashMap`, 64-bit integers, and enum representations that differ
//! between the two serializers.

use folco_model::{RectPx, SurfaceColor};
use folco_renderer::{FolderIconBase, IconImage, IconSet};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// PNG-encoded representation of an icon image for wasm transfer.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct SerializableIconImage {
    /// PNG-encoded image bytes.
    pub png_data: Vec<u8>,
    /// Display scale factor (1.0 for @1x, 2.0 for @2x, etc.).
    pub scale: f32,
    /// Pixel width of the image.
    pub width: u32,
    /// Pixel height of the image.
    pub height: u32,
    /// The region within the image containing the actual icon content.
    #[serde(default)]
    pub content_bounds: Option<RectPx>,
}

/// Serializable representation of a [`FolderIconBase`] for wasm transfer.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct SerializableFolderIconBase {
    /// PNG-encoded icon images at various sizes/scales.
    pub images: Vec<SerializableIconImage>,
    /// The surface color of the base icon.
    pub surface_color: SurfaceColor,
}

impl TryFrom<&IconImage> for SerializableIconImage {
    type Error = image::ImageError;

    fn try_from(img: &IconImage) -> Result<Self, Self::Error> {
        Ok(Self {
            png_data: img.to_png_bytes()?,
            scale: img.scale,
            width: img.data.width(),
            height: img.data.height(),
            content_bounds: Some(img.content_bounds),
        })
    }
}

impl TryFrom<&FolderIconBase> for SerializableFolderIconBase {
    type Error = image::ImageError;

    fn try_from(base: &FolderIconBase) -> Result<Self, Self::Error> {
        let images = base
            .icons
            .iter()
            .map(SerializableIconImage::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            images,
            surface_color: base.surface_color,
        })
    }
}

impl SerializableFolderIconBase {
    /// Decodes the PNG images and reconstructs a [`FolderIconBase`].
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
