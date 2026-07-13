//! Shared serializable transfer types for the folder-icon-base boundary.
//!
//! These are the PNG-encoded transfer types used to move a [`FolderIconBase`]
//! across process/language boundaries. They are defined **once** here and
//! reused by both:
//! - the Tauri backend, which serializes them with `serde_json` for IPC, and
//! - the wasm renderer bindings, which deserialize them with
//!   `serde-wasm-bindgen` (via `tsify`) and feed them into
//!   `CanvasRenderer::from_folder_icon_base`.
//!
//! Because a single definition backs both paths, the two serializers are
//! guaranteed to agree on the wire shape — there is no longer a hand-maintained
//! "keep these in sync" contract. When adding fields, still avoid types whose
//! representation differs between `serde_json` and `serde-wasm-bindgen`
//! (e.g. `HashMap`, 64-bit integers, unusual enum representations).
//!
//! The `tsify` feature is opt-in (mirroring `folco-model`): the wasm crate
//! enables it to derive [`tsify::Tsify`] and the wasm-ABI conversions, while
//! the Tauri app leaves it off and gets plain serde.

use folco_model::{RectPx, SurfaceColor};
use folco_renderer::{FolderIconBase, IconImage, IconSet};
use serde::{Deserialize, Serialize};

/// PNG-encoded representation of an icon image for boundary transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
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

/// Serializable representation of a [`FolderIconBase`] for boundary transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
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
