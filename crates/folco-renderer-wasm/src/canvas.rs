//! HTML Canvas rendering for WASM environments.
//!
//! This crate provides [`CanvasRenderer`], a live-preview wrapper that renders
//! folder icons to an HTML canvas. It works for both icon media — raster (PNG
//! sets) and vector (SVG) — dispatching internally, so callers configure layers
//! the same way regardless of what the platform provided.
//!
//! Previews always rasterize; the artifact written to the system is medium
//! native and comes from [`CanvasRenderer::render_output_svg`] (vector) or the
//! backend's raster pipeline.
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
//! // Update the folder color and render
//! renderer.setSolidColor(33, 150, 243);
//! renderer.renderToCanvas(canvas, 256);
//!
//! // Export profile when done
//! const profileJson = renderer.export_profile_json();
//! ```

use wasm_bindgen::Clamped;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

use folco_renderer::{
    ColorDotConfig, CustomizationProfile, DecalConfig, FolderIconBase, FolderIconCustomizer,
    IconImage, IconSet, ImageOverlayConfig, OverlayAnchorMode, OverlayPosition, RectPx,
    SolidColorConfig, SurfaceColor, SvgFolderIconBase, SvgFolderIconCustomizer,
};

use folco_transfer::{SerializableFolderIconBase, SerializableSvgFolderIconBase};

fn parse_overlay_position(position: &str) -> OverlayPosition {
    match position {
        "top-left" => OverlayPosition::TopLeft,
        "top-right" => OverlayPosition::TopRight,
        "bottom-left" => OverlayPosition::BottomLeft,
        "center" => OverlayPosition::Center,
        _ => OverlayPosition::BottomRight,
    }
}

pub(crate) fn parse_overlay_anchor_mode(anchor_mode: &str) -> OverlayAnchorMode {
    match anchor_mode {
        "centered" => OverlayAnchorMode::Centered,
        _ => OverlayAnchorMode::Inset,
    }
}

/// Resizes `canvas` to the image and blits its pixels into the 2d context.
fn draw_rgba_to_canvas(canvas: &HtmlCanvasElement, image: image::RgbaImage) -> Result<(), JsError> {
    let width = image.width();
    let height = image.height();

    canvas.set_width(width);
    canvas.set_height(height);

    let ctx: CanvasRenderingContext2d = canvas
        .get_context("2d")
        .map_err(|_| JsError::new("Failed to get 2d context"))?
        .ok_or_else(|| JsError::new("Canvas 2d context is null"))?
        .dyn_into()
        .map_err(|_| JsError::new("Failed to cast to CanvasRenderingContext2d"))?;

    let raw_pixels: Vec<u8> = image.into_raw();
    let image_data =
        ImageData::new_with_u8_clamped_array_and_sh(Clamped(&raw_pixels), width, height)
            .map_err(|_| JsError::new("Failed to create ImageData"))?;

    ctx.put_image_data(&image_data, 0.0, 0.0)
        .map_err(|_| JsError::new("Failed to put image data"))?;

    Ok(())
}

// ============================================================================
// CanvasRenderer
// ============================================================================

/// The active customizer, selected by the medium of the base icon.
enum Medium {
    Raster(Box<FolderIconCustomizer>),
    Svg(Box<SvgFolderIconCustomizer>),
}

/// A live-preview folder icon renderer targeting an HTML canvas element.
///
/// Wraps whichever customizer matches the base icon's medium and exposes one
/// layer-configuration API over both. This type is exposed to JavaScript via
/// wasm-bindgen.
#[wasm_bindgen]
pub struct CanvasRenderer {
    medium: Medium,
}

impl CanvasRenderer {
    fn raster(customizer: FolderIconCustomizer) -> Self {
        Self {
            medium: Medium::Raster(Box::new(customizer)),
        }
    }

    fn set_overlay_config(&mut self, config: Option<ImageOverlayConfig>) {
        let enabled = config.is_some();
        match &mut self.medium {
            Medium::Raster(c) => {
                c.layers.overlay.set_config(config);
                c.layers.overlay.set_enabled(enabled);
            }
            Medium::Svg(c) => {
                c.layers.overlay.set_config(config);
                c.layers.overlay.set_enabled(enabled);
            }
        }
    }

    fn render_preview(&mut self, size: u32) -> Result<IconImage, JsError> {
        match &mut self.medium {
            Medium::Raster(c) => c.render(size),
            Medium::Svg(c) => c.render_preview(size),
        }
        .map_err(|e| JsError::new(&e.to_string()))
    }
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

        Ok(Self::raster(FolderIconCustomizer::from_folder(
            FolderIconBase::new(icon_set, surface_color),
        )))
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

        Ok(Self::raster(FolderIconCustomizer::from_folder(
            FolderIconBase::new(icon_set, surface_color),
        )))
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

        Ok(Self::raster(FolderIconCustomizer::from_folder(base)))
    }

    /// Creates a new renderer from a scalable (SVG) folder icon base.
    ///
    /// Accepts the DTO the Tauri backend sends for vector folder icons.
    #[wasm_bindgen(js_name = "fromSvgFolderIconBase")]
    pub fn from_svg_folder_icon_base(base: SerializableSvgFolderIconBase) -> CanvasRenderer {
        let customizer = SvgFolderIconCustomizer::from_folder(SvgFolderIconBase::new(
            base.svg,
            base.surface_color,
        ));
        Self {
            medium: Medium::Svg(Box::new(customizer)),
        }
    }

    /// Returns `true` if the active medium supports the decal layer.
    ///
    /// Vector icons have no decal layer yet, so UIs should disable the control
    /// rather than let it silently do nothing.
    #[wasm_bindgen(js_name = "supportsDecal")]
    pub fn supports_decal(&self) -> bool {
        matches!(self.medium, Medium::Raster(_))
    }

    /// Returns `true` if the active medium supports the solid color layer.
    ///
    /// Recoloring works by HSL-shifting pixels against a known surface color,
    /// which an arbitrary SVG can't be put through. Vector icons offer the
    /// color dot instead, so UIs should disable the control rather than let it
    /// silently do nothing.
    #[wasm_bindgen(js_name = "supportsSolidColor")]
    pub fn supports_solid_color(&self) -> bool {
        matches!(self.medium, Medium::Raster(_))
    }

    /// Returns `true` if the active medium is vector (SVG).
    #[wasm_bindgen(js_name = "isSvg")]
    pub fn is_svg(&self) -> bool {
        matches!(self.medium, Medium::Svg(_))
    }

    // ---- Layer Configuration ----

    /// Recolors the whole icon to a target RGB color.
    ///
    /// Ignored for vector icons — see [`supports_solid_color`](Self::supports_solid_color).
    ///
    /// # Arguments
    ///
    /// * `target_r` - Target red channel (0–255)
    /// * `target_g` - Target green channel (0–255)
    /// * `target_b` - Target blue channel (0–255)
    #[wasm_bindgen(js_name = "setSolidColor")]
    pub fn set_solid_color(&mut self, target_r: u8, target_g: u8, target_b: u8) {
        if let Medium::Raster(c) = &mut self.medium {
            c.layers
                .solid_color
                .set_config(Some(SolidColorConfig::new(target_r, target_g, target_b)));
            c.layers.solid_color.set_enabled(true);
        }
    }

    /// Sets the solid color enabled state without changing the parameters.
    #[wasm_bindgen(js_name = "setSolidColorEnabled")]
    pub fn set_solid_color_enabled(&mut self, enabled: bool) {
        if let Medium::Raster(c) = &mut self.medium {
            c.layers.solid_color.set_enabled(enabled);
        }
    }

    /// Sets the color-dot badge drawn in the icon's bottom-right corner.
    ///
    /// Supported by every medium.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel (0–255)
    /// * `g` - Green channel (0–255)
    /// * `b` - Blue channel (0–255)
    #[wasm_bindgen(js_name = "setColorDot")]
    pub fn set_color_dot(&mut self, r: u8, g: u8, b: u8) {
        let config = ColorDotConfig::new(r, g, b);
        match &mut self.medium {
            Medium::Raster(c) => {
                c.layers.color_dot.set_config(Some(config));
                c.layers.color_dot.set_enabled(true);
            }
            Medium::Svg(c) => {
                c.layers.color_dot.set_config(Some(config));
                c.layers.color_dot.set_enabled(true);
            }
        }
    }

    /// Sets the color dot enabled state without changing the parameters.
    #[wasm_bindgen(js_name = "setColorDotEnabled")]
    pub fn set_color_dot_enabled(&mut self, enabled: bool) {
        match &mut self.medium {
            Medium::Raster(c) => c.layers.color_dot.set_enabled(enabled),
            Medium::Svg(c) => c.layers.color_dot.set_enabled(enabled),
        };
    }

    /// Sets the decal configuration.
    ///
    /// Ignored for vector icons — see [`supports_decal`](Self::supports_decal).
    ///
    /// # Arguments
    ///
    /// * `svg_data` - The SVG string for the decal, or `null` to clear it
    /// * `scale` - Scale factor relative to icon bounds (0.0-1.0)
    #[wasm_bindgen(js_name = "setDecal")]
    pub fn set_decal(&mut self, svg_data: Option<String>, scale: f32) {
        let Medium::Raster(c) = &mut self.medium else {
            return;
        };
        match svg_data {
            Some(svg) if !svg.is_empty() => {
                c.layers
                    .decal
                    .set_config(Some(DecalConfig::new(svg, scale)));
                c.layers.decal.set_enabled(true);
            }
            _ => {
                c.layers.decal.set_config(None);
                c.layers.decal.set_enabled(false);
            }
        }
    }

    /// Sets the decal enabled state without changing the configuration.
    #[wasm_bindgen(js_name = "setDecalEnabled")]
    pub fn set_decal_enabled(&mut self, enabled: bool) {
        if let Medium::Raster(c) = &mut self.medium {
            c.layers.decal.set_enabled(enabled);
        }
    }

    /// Sets the overlay configuration.
    ///
    /// Passing `null` clears the stored source, so a later `setOverlayEnabled(true)`
    /// cannot bring back a previously selected icon or emoji.
    ///
    /// # Arguments
    ///
    /// * `svg_data` - The SVG string for the overlay, or `null` to clear it
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

        let config = match svg_data {
            Some(svg) if !svg.is_empty() => {
                Some(ImageOverlayConfig::from_svg(svg, pos, anchor_mode, scale))
            }
            _ => None,
        };

        self.set_overlay_config(config);
    }

    /// Sets the overlay enabled state without changing the configuration.
    #[wasm_bindgen(js_name = "setOverlayEnabled")]
    pub fn set_overlay_enabled(&mut self, enabled: bool) {
        match &mut self.medium {
            Medium::Raster(c) => c.layers.overlay.set_enabled(enabled),
            Medium::Svg(c) => c.layers.overlay.set_enabled(enabled),
        };
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
        self.set_overlay_config(Some(config));
        Ok(())
    }

    // ---- Rendering ----

    /// Renders a rasterized preview to an HTML canvas element.
    ///
    /// # Arguments
    ///
    /// * `canvas` - The target canvas element
    /// * `size` - The logical size to render (raster picks the closest available)
    #[wasm_bindgen(js_name = "renderToCanvas")]
    pub fn render_to_canvas(
        &mut self,
        canvas: &HtmlCanvasElement,
        size: u32,
    ) -> Result<(), JsError> {
        let rendered = self.render_preview(size)?;
        draw_rgba_to_canvas(canvas, rendered.data)
    }

    /// Renders a rasterized preview and returns raw RGBA pixel data.
    ///
    /// Useful if you need to manipulate the pixels in JavaScript before drawing.
    #[wasm_bindgen(js_name = "renderToPixels")]
    pub fn render_to_pixels(&mut self, size: u32) -> Result<js_sys::Uint8Array, JsError> {
        let rendered = self.render_preview(size)?;

        let raw_pixels = rendered.data.into_raw();
        let array = js_sys::Uint8Array::new_with_length(raw_pixels.len() as u32);
        array.copy_from(&raw_pixels);
        Ok(array)
    }

    /// Returns the dimensions of the rendered preview at the given logical size.
    #[wasm_bindgen(js_name = "getRenderedDimensions")]
    pub fn get_rendered_dimensions(&self, size: u32) -> Result<js_sys::Array, JsError> {
        let (width, height) = match &self.medium {
            Medium::Raster(c) => {
                let icon = c
                    .base_icons()
                    .find_by_logical_size(size)
                    .ok_or_else(|| JsError::new("No icon available at requested size"))?;
                (icon.data.width(), icon.data.height())
            }
            // Vector icons are resolution-independent, so any size is valid.
            Medium::Svg(_) => (size, size),
        };

        let arr = js_sys::Array::new();
        arr.push(&JsValue::from(width));
        arr.push(&JsValue::from(height));
        Ok(arr)
    }

    /// Renders the scalable artifact written to the system, for vector icons.
    ///
    /// Returns `null` for raster icons, where the saved artifact is a PNG set
    /// produced by the backend.
    #[wasm_bindgen(js_name = "renderOutputSvg")]
    pub fn render_output_svg(&mut self) -> Result<Option<String>, JsError> {
        match &mut self.medium {
            Medium::Raster(_) => Ok(None),
            Medium::Svg(c) => c
                .render_output()
                .map(Some)
                .map_err(|e| JsError::new(&e.to_string())),
        }
    }

    // ---- Profile Import/Export ----

    /// Exports the current settings as a JSON string.
    #[wasm_bindgen(js_name = "exportProfileJson")]
    pub fn export_profile_json(&self) -> Result<String, JsError> {
        let profile = match &self.medium {
            Medium::Raster(c) => c.export_profile(),
            Medium::Svg(c) => c.export_profile(),
        };
        profile
            .to_json()
            .map_err(|e| JsError::new(&format!("Failed to serialize profile: {}", e)))
    }

    /// Imports settings from a JSON string.
    #[wasm_bindgen(js_name = "importProfileJson")]
    pub fn import_profile_json(&mut self, json: &str) -> Result<(), JsError> {
        let profile = CustomizationProfile::from_json(json)
            .map_err(|e| JsError::new(&format!("Failed to parse profile: {}", e)))?;
        match &mut self.medium {
            Medium::Raster(c) => c.apply_profile(&profile),
            Medium::Svg(c) => c.apply_profile(&profile),
        }
        Ok(())
    }

    /// Clears all customizations and returns to the base icon.
    pub fn reset(&mut self) {
        match &mut self.medium {
            Medium::Raster(c) => {
                c.layers.solid_color.set_config(None);
                c.layers.color_dot.set_config(None);
                c.layers.decal.set_config(None);
                c.layers.overlay.set_config(None);
            }
            Medium::Svg(c) => {
                c.layers.color_dot.set_config(None);
                c.layers.overlay.set_config(None);
            }
        }
    }

    /// Clears the render cache to free memory.
    #[wasm_bindgen(js_name = "clearCache")]
    pub fn clear_cache(&mut self) {
        match &mut self.medium {
            Medium::Raster(c) => c.clear_cache(),
            Medium::Svg(c) => c.clear_cache(),
        }
    }
}
