//! HTML Canvas rendering for WASM environments.
//!
//! This crate provides [`CanvasRenderer`], a wrapper around [`FolderIconCustomizer`]
//! that can render directly to an HTML canvas element. It's designed for
//! live preview in web frontends.
//!
//! # Example (JavaScript/TypeScript)
//!
//! ```javascript
//! import init, { CanvasRenderer } from 'folco-renderer-wasm';
//!
//! await init();
//!
//! // Get the canvas element
//! const canvas = document.getElementById('preview-canvas');
//!
//! // Create renderer with base icon (as Uint8Array PNG data)
//! // Surface color is the folder icon's base RGB color (e.g. Windows: 255, 217, 112)
//! const renderer = CanvasRenderer.fromPng(baseIconPng, 1.0, 255, 217, 112);
//!
//! // Update color target and render
//! renderer.setFolderColorTarget(33, 150, 243);
//! renderer.renderToCanvas(canvas, 256);
//!
//! // Export profile when done
//! const profileJson = renderer.export_profile_json();
//! ```

use wasm_bindgen::Clamped;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use folco_renderer::CustomizationProfile;
use folco_renderer::FolderIconCustomizer;
use folco_renderer::{
    DecalConfig, FolderColorTargetConfig, ImageOverlayConfig, OverlayAnchorMode, OverlayPosition,
};
use folco_renderer::{
    FolderIconBase, IconImage, IconSet, RectPx, SerializableFolderIconBase, SurfaceColor,
};

fn parse_overlay_position(position: &str) -> OverlayPosition {
    match position {
        "top-left" => OverlayPosition::TopLeft,
        "top-right" => OverlayPosition::TopRight,
        "bottom-left" => OverlayPosition::BottomLeft,
        "center" => OverlayPosition::Center,
        _ => OverlayPosition::BottomRight,
    }
}

fn parse_overlay_anchor_mode(anchor_mode: &str) -> OverlayAnchorMode {
    match anchor_mode {
        "centered" => OverlayAnchorMode::Centered,
        _ => OverlayAnchorMode::Inset,
    }
}

// ============================================================================
// CanvasRenderer
// ============================================================================

/// A wrapper around [`FolderIconCustomizer`] for rendering to HTML canvas elements.
///
/// This type is exposed to JavaScript via wasm-bindgen and provides a simple
/// API for live preview in web UIs.
#[wasm_bindgen]
pub struct CanvasRenderer {
    customizer: FolderIconCustomizer,
}

#[wasm_bindgen]
impl CanvasRenderer {
    /// Creates a new renderer from PNG image data.
    ///
    /// # Arguments
    ///
    /// * `png_data` - The raw PNG bytes of the base icon
    /// * `scale` - The display scale factor (1.0 for @1x, 2.0 for @2x, etc.)
    /// * `surface_r` - Surface color red channel (0–255)
    /// * `surface_g` - Surface color green channel (0–255)
    /// * `surface_b` - Surface color blue channel (0–255)
    ///
    /// # Returns
    ///
    /// A new `CanvasRenderer`, or an error if the PNG cannot be decoded.
    #[wasm_bindgen(js_name = "fromPng")]
    pub fn from_png(
        png_data: &[u8],
        scale: f32,
        surface_r: u8,
        surface_g: u8,
        surface_b: u8,
    ) -> Result<CanvasRenderer, JsError> {
        let surface_color = SurfaceColor::new(surface_r, surface_g, surface_b);
        let img = image::load_from_memory(png_data)
            .map_err(|e| JsError::new(&format!("Failed to decode PNG: {}", e)))?
            .to_rgba8();

        let width = img.width();
        let height = img.height();
        let icon = IconImage::new(img, scale, RectPx::from_size(width, height));

        let mut icon_set = IconSet::new();
        icon_set.add_image(icon);

        Ok(Self {
            customizer: FolderIconCustomizer::from_folder(FolderIconBase::new(
                icon_set,
                surface_color,
            )),
        })
    }

    /// Creates a new renderer from multiple PNG images (for multi-resolution icons).
    ///
    /// # Arguments
    ///
    /// * `png_data_array` - Array of PNG byte arrays
    /// * `scales` - Array of scale factors corresponding to each PNG
    /// * `surface_r` - Surface color red channel (0–255)
    /// * `surface_g` - Surface color green channel (0–255)
    /// * `surface_b` - Surface color blue channel (0–255)
    #[wasm_bindgen(js_name = "fromPngMultiple")]
    pub fn from_png_multiple(
        png_data_array: js_sys::Array,
        scales: &[f32],
        surface_r: u8,
        surface_g: u8,
        surface_b: u8,
    ) -> Result<CanvasRenderer, JsError> {
        let surface_color = SurfaceColor::new(surface_r, surface_g, surface_b);
        let mut icon_set = IconSet::new();

        for (i, scale) in scales.iter().enumerate() {
            let png_data: js_sys::Uint8Array = png_data_array
                .get(i as u32)
                .dyn_into()
                .map_err(|_| JsError::new(&format!("Expected Uint8Array at index {}", i)))?;

            let bytes = png_data.to_vec();
            let img = image::load_from_memory(&bytes)
                .map_err(|e| JsError::new(&format!("Failed to decode PNG at index {}: {}", i, e)))?
                .to_rgba8();

            let width = img.width();
            let height = img.height();
            let icon = IconImage::new(img, *scale, RectPx::from_size(width, height));
            icon_set.add_image(icon);
        }

        Ok(Self {
            customizer: FolderIconCustomizer::from_folder(FolderIconBase::new(
                icon_set,
                surface_color,
            )),
        })
    }

    /// Creates a new renderer from a [`SerializableFolderIconBase`].
    ///
    /// This accepts the same DTO that the Tauri backend sends over IPC,
    /// so the frontend can pass it straight through without unpacking fields.
    ///
    /// # Arguments
    ///
    /// * `folder_icon_base` - A serializable icon base (PNG-encoded images + surface color)
    #[wasm_bindgen(js_name = "fromFolderIconBase")]
    pub fn from_folder_icon_base(
        folder_icon_base: SerializableFolderIconBase,
    ) -> Result<CanvasRenderer, JsError> {
        let base = folder_icon_base
            .into_folder_icon_base()
            .map_err(|e| JsError::new(&format!("Failed to decode icon base: {e}")))?;

        Ok(Self {
            customizer: FolderIconCustomizer::from_folder(base),
        })
    }

    // ---- Layer Configuration ----

    /// Sets the color target from a target RGB color.
    ///
    /// The renderer computes the necessary HSL deltas from the base surface color
    /// to the target color internally.
    ///
    /// # Arguments
    ///
    /// * `target_r` - Target red channel (0–255), or `null` to disable
    /// * `target_g` - Target green channel (0–255)
    /// * `target_b` - Target blue channel (0–255)
    #[wasm_bindgen(js_name = "setFolderColorTarget")]
    pub fn set_folder_color_target(&mut self, target_r: u8, target_g: u8, target_b: u8) {
        self.customizer
            .layers
            .folder_color_target
            .set_config(Some(FolderColorTargetConfig::new(
                target_r, target_g, target_b,
            )));
        self.customizer.layers.folder_color_target.set_enabled(true);
    }

    /// Sets the color target enabled state without changing the parameters.
    #[wasm_bindgen(js_name = "setFolderColorTargetEnabled")]
    pub fn set_folder_color_target_enabled(&mut self, enabled: bool) {
        self.customizer
            .layers
            .folder_color_target
            .set_enabled(enabled);
    }

    /// Sets the decal configuration.
    ///
    /// # Arguments
    ///
    /// * `svg_data` - The SVG string for the decal, or `null` to disable
    /// * `scale` - Scale factor relative to icon bounds (0.0-1.0)
    #[wasm_bindgen(js_name = "setDecal")]
    pub fn set_decal(&mut self, svg_data: Option<String>, scale: f32) {
        match svg_data {
            Some(svg) if !svg.is_empty() => {
                self.customizer
                    .layers
                    .decal
                    .set_config(Some(DecalConfig::new(svg, scale)));
                self.customizer.layers.decal.set_enabled(true);
            }
            _ => {
                self.customizer.layers.decal.set_enabled(false);
            }
        }
    }

    /// Sets the decal enabled state without changing the configuration.
    #[wasm_bindgen(js_name = "setDecalEnabled")]
    pub fn set_decal_enabled(&mut self, enabled: bool) {
        self.customizer.layers.decal.set_enabled(enabled);
    }

    /// Sets the overlay configuration.
    ///
    /// # Arguments
    ///
    /// * `svg_data` - The SVG string for the overlay, or `null` to disable
    /// * `position` - Position string: "top-left", "top-right", "bottom-left", "bottom-right", "center"
    /// * `anchor_mode` - Anchor mode string: "inset" or "centered"
    /// * `scale` - Scale factor relative to icon bounds (0.0-1.0)
    #[wasm_bindgen(js_name = "setOverlay")]
    pub fn set_overlay(
        &mut self,
        svg_data: Option<String>,
        position: &str,
        anchor_mode: &str,
        scale: f32,
    ) {
        let pos = parse_overlay_position(position);
        let anchor_mode = parse_overlay_anchor_mode(anchor_mode);

        match svg_data {
            Some(svg) if !svg.is_empty() => {
                self.customizer
                    .layers
                    .overlay
                    .set_config(Some(ImageOverlayConfig::from_svg(
                        svg,
                        pos,
                        anchor_mode,
                        scale,
                    )));
                self.customizer.layers.overlay.set_enabled(true);
            }
            _ => {
                self.customizer.layers.overlay.set_enabled(false);
            }
        }
    }

    /// Sets the overlay enabled state without changing the configuration.
    #[wasm_bindgen(js_name = "setOverlayEnabled")]
    pub fn set_overlay_enabled(&mut self, enabled: bool) {
        self.customizer.layers.overlay.set_enabled(enabled);
    }

    /// Sets the overlay to an emoji character.
    ///
    /// Returns an error if the emoji is not supported.
    ///
    /// # Arguments
    ///
    /// * `emoji` - The emoji character (e.g., "🦆")
    /// * `position` - Position string: "top-left", "top-right", "bottom-left", "bottom-right", "center"
    /// * `anchor_mode` - Anchor mode string: "inset" or "centered"
    /// * `scale` - Scale factor relative to icon bounds (0.0-1.0)
    #[cfg(feature = "twemoji")]
    #[wasm_bindgen(js_name = "setOverlayEmoji")]
    pub fn set_overlay_emoji(
        &mut self,
        emoji: &str,
        position: &str,
        anchor_mode: &str,
        scale: f32,
    ) -> Result<(), JsError> {
        let pos = parse_overlay_position(position);
        let anchor_mode = parse_overlay_anchor_mode(anchor_mode);
        let config = ImageOverlayConfig::from_emoji(emoji, pos, anchor_mode, scale)
            .map_err(|e| JsError::new(&e.to_string()))?;
        self.customizer.layers.overlay.set_config(Some(config));
        self.customizer.layers.overlay.set_enabled(true);
        Ok(())
    }

    // ---- Rendering ----

    /// Renders the customized icon to an HTML canvas element.
    ///
    /// # Arguments
    ///
    /// * `canvas` - The target canvas element
    /// * `size` - The logical size to render (will pick closest available size)
    #[wasm_bindgen(js_name = "renderToCanvas")]
    pub fn render_to_canvas(
        &mut self,
        canvas: &HtmlCanvasElement,
        size: u32,
    ) -> Result<(), JsError> {
        let rendered = self
            .customizer
            .render(size)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let width = rendered.data.width();
        let height = rendered.data.height();

        // Resize canvas to match rendered size
        canvas.set_width(width);
        canvas.set_height(height);

        let ctx: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .map_err(|_| JsError::new("Failed to get 2d context"))?
            .ok_or_else(|| JsError::new("Canvas 2d context is null"))?
            .dyn_into()
            .map_err(|_| JsError::new("Failed to cast to CanvasRenderingContext2d"))?;

        // Convert RGBA image to ImageData
        let raw_pixels: Vec<u8> = rendered.data.into_raw();
        let image_data =
            ImageData::new_with_u8_clamped_array_and_sh(Clamped(&raw_pixels), width, height)
                .map_err(|_| JsError::new("Failed to create ImageData"))?;

        // Draw to canvas
        ctx.put_image_data(&image_data, 0.0, 0.0)
            .map_err(|_| JsError::new("Failed to put image data"))?;

        Ok(())
    }

    /// Renders the customized icon and returns raw RGBA pixel data.
    ///
    /// Useful if you need to manipulate the pixels in JavaScript before drawing.
    #[wasm_bindgen(js_name = "renderToPixels")]
    pub fn render_to_pixels(&mut self, size: u32) -> Result<js_sys::Uint8Array, JsError> {
        let rendered = self
            .customizer
            .render(size)
            .map_err(|e| JsError::new(&e.to_string()))?;

        let raw_pixels = rendered.data.into_raw();
        let array = js_sys::Uint8Array::new_with_length(raw_pixels.len() as u32);
        array.copy_from(&raw_pixels);
        Ok(array)
    }

    /// Returns the dimensions of the rendered icon at the given logical size.
    #[wasm_bindgen(js_name = "getRenderedDimensions")]
    pub fn get_rendered_dimensions(&self, size: u32) -> Result<js_sys::Array, JsError> {
        let icon = self
            .customizer
            .base_icons()
            .find_by_logical_size(size)
            .ok_or_else(|| JsError::new("No icon available at requested size"))?;

        let arr = js_sys::Array::new();
        arr.push(&JsValue::from(icon.data.width()));
        arr.push(&JsValue::from(icon.data.height()));
        Ok(arr)
    }

    // ---- Profile Import/Export ----

    /// Exports the current settings as a JSON string.
    #[wasm_bindgen(js_name = "exportProfileJson")]
    pub fn export_profile_json(&self) -> Result<String, JsError> {
        let profile = self.customizer.export_profile();
        profile
            .to_json()
            .map_err(|e| JsError::new(&format!("Failed to serialize profile: {}", e)))
    }

    /// Imports settings from a JSON string.
    #[wasm_bindgen(js_name = "importProfileJson")]
    pub fn import_profile_json(&mut self, json: &str) -> Result<(), JsError> {
        let profile = CustomizationProfile::from_json(json)
            .map_err(|e| JsError::new(&format!("Failed to parse profile: {}", e)))?;
        self.customizer.apply_profile(&profile);
        Ok(())
    }

    /// Clears all customizations and returns to the base icon.
    pub fn reset(&mut self) {
        self.customizer.layers.folder_color_target.set_config(None);
        self.customizer.layers.decal.set_config(None);
        self.customizer.layers.overlay.set_config(None);
    }

    /// Clears the render cache to free memory.
    #[wasm_bindgen(js_name = "clearCache")]
    pub fn clear_cache(&mut self) {
        self.customizer.clear_cache();
    }
}
