//! HTML Canvas rendering for WASM environments.
//!
//! This crate provides [`CanvasRenderer`], a live-preview wrapper that renders
//! icons to an HTML canvas. It covers every base a customization can start
//! from — the system folder icon (raster or vector) and a user-supplied image —
//! dispatching internally, so callers configure layers the same way regardless
//! of what they are working on.
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
    ColorDotConfig, CustomIconCustomizer, CustomizationProfile, DecalConfig, FolderIconBase,
    FolderIconCustomizer, IconBaseKind, IconCapabilities, IconImage, IconSet, IconSizeSpec,
    ImageOverlayConfig, ImageSource, LayerRejections, OverlayAnchorMode, OverlayPosition, RectPx,
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

/// The customizer backing this renderer, chosen at construction.
///
/// Folder icons take whichever medium the platform provides; custom icons are
/// always pixels, because the user supplied them. `Folder` and `Custom` are
/// both raster — they differ in whether a surface color exists to shift
/// against, which is what gates the solid-color and decal layers.
enum ActiveCustomizer {
    /// Pixel pipeline over the system's PNG folder icons.
    Folder(Box<FolderIconCustomizer>),
    /// Vector pipeline over the system's scalable SVG folder icon.
    SvgFolder(Box<SvgFolderIconCustomizer>),
    /// Pixel pipeline over a user-supplied image.
    Custom(Box<CustomIconCustomizer>),
}

/// A live-preview icon renderer targeting an HTML canvas element.
///
/// Wraps whichever customizer matches the icon's origin and medium, and exposes
/// one layer-configuration API over all of them. Layers the active customizer
/// can't realize are ignored, so callers configure the same way regardless;
/// query the `supports*` flags to disable controls rather than let them
/// silently do nothing. This type is exposed to JavaScript via wasm-bindgen.
#[wasm_bindgen]
pub struct CanvasRenderer {
    active: ActiveCustomizer,
}

impl CanvasRenderer {
    fn folder(customizer: FolderIconCustomizer) -> Self {
        Self {
            active: ActiveCustomizer::Folder(Box::new(customizer)),
        }
    }

    /// Builds a custom-icon renderer by rasterizing `source` to each spec.
    fn from_custom_source(
        source: ImageSource,
        size_specs: JsValue,
    ) -> Result<CanvasRenderer, JsError> {
        let specs: Vec<IconSizeSpec> = serde_wasm_bindgen::from_value(size_specs)
            .map_err(|e| JsError::new(&format!("Failed to parse icon size specs: {e}")))?;

        // An empty set would build a renderer whose every render fails later.
        if specs.is_empty() {
            return Err(JsError::new("At least one icon size spec is required"));
        }

        let customizer = CustomIconCustomizer::from_image(&source, &specs)
            .map_err(|e| JsError::new(&format!("Failed to build custom icon: {e}")))?;

        Ok(Self {
            active: ActiveCustomizer::Custom(Box::new(customizer)),
        })
    }

    fn set_overlay_config(&mut self, config: Option<ImageOverlayConfig>) {
        let enabled = config.is_some();
        match &mut self.active {
            ActiveCustomizer::Folder(c) => {
                c.layers.overlay.set_config(config);
                c.layers.overlay.set_enabled(enabled);
            }
            ActiveCustomizer::SvgFolder(c) => {
                c.layers.overlay.set_config(config);
                c.layers.overlay.set_enabled(enabled);
            }
            ActiveCustomizer::Custom(c) => {
                c.layers.overlay.set_config(config);
                c.layers.overlay.set_enabled(enabled);
            }
        }
    }

    fn render_preview(&mut self, size: u32) -> Result<IconImage, JsError> {
        match &mut self.active {
            ActiveCustomizer::Folder(c) => c.render(size),
            ActiveCustomizer::SvgFolder(c) => c.render_preview(size),
            ActiveCustomizer::Custom(c) => c.render(size),
        }
        .map_err(|e| JsError::new(&e.to_string()))
    }

    fn base_dimensions(icons: &IconSet, size: u32) -> Result<(u32, u32), JsError> {
        let icon = icons
            .find_by_logical_size(size)
            .ok_or_else(|| JsError::new("No icon available at requested size"))?;
        Ok((icon.data.width(), icon.data.height()))
    }

    fn base_kind(&self) -> IconBaseKind {
        match &self.active {
            ActiveCustomizer::Folder(_) => FolderIconCustomizer::base_kind(),
            ActiveCustomizer::SvgFolder(_) => SvgFolderIconCustomizer::base_kind(),
            ActiveCustomizer::Custom(_) => CustomIconCustomizer::base_kind(),
        }
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

        Ok(Self::folder(FolderIconCustomizer::from_folder(
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

        Ok(Self::folder(FolderIconCustomizer::from_folder(
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

        Ok(Self::folder(FolderIconCustomizer::from_folder(base)))
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
            active: ActiveCustomizer::SvgFolder(Box::new(customizer)),
        }
    }

    /// Creates a renderer for a user-supplied raster image.
    ///
    /// The image is decoded and resized to each size spec up front, so pass the
    /// platform's required sizes (from the backend's `get_platform_icon_sizes`)
    /// rather than inventing a ladder here.
    ///
    /// # Arguments
    ///
    /// * `image_data` - Encoded image bytes (PNG, JPEG, WebP, ...)
    /// * `size_specs` - Array of `IconSizeSpec` objects; must be non-empty
    #[wasm_bindgen(js_name = "fromCustomImage")]
    pub fn from_custom_image(
        image_data: Vec<u8>,
        #[wasm_bindgen(unchecked_param_type = "IconSizeSpec[]")] size_specs: JsValue,
    ) -> Result<CanvasRenderer, JsError> {
        Self::from_custom_source(ImageSource::raster(image_data), size_specs)
    }

    /// Creates a renderer for a user-supplied SVG image.
    ///
    /// The markup is rasterized to each size spec up front. The result is a
    /// raster icon: this is a vector *source*, not the vector pipeline that
    /// [`from_svg_folder_icon_base`](Self::from_svg_folder_icon_base) uses.
    ///
    /// # Arguments
    ///
    /// * `svg` - Raw SVG markup
    /// * `size_specs` - Array of `IconSizeSpec` objects; must be non-empty
    #[wasm_bindgen(js_name = "fromCustomSvg")]
    pub fn from_custom_svg(
        svg: String,
        #[wasm_bindgen(unchecked_param_type = "IconSizeSpec[]")] size_specs: JsValue,
    ) -> Result<CanvasRenderer, JsError> {
        Self::from_custom_source(ImageSource::svg(svg), size_specs)
    }

    /// Returns `true` if the active customizer supports the decal layer.
    ///
    /// Decals imprint per-pixel and tint against the base's surface color, so
    /// only system folder icons in raster form qualify. UIs should disable the
    /// control rather than let it silently do nothing.
    #[wasm_bindgen(js_name = "supportsDecal")]
    pub fn supports_decal(&self) -> bool {
        self.capabilities().decal
    }

    /// Returns `true` if the active customizer supports the solid color layer.
    ///
    /// Recoloring works by HSL-shifting pixels against a known surface color.
    /// An arbitrary SVG has no pixels to shift, and a user-supplied image has
    /// no surface color to shift against, so only raster folder icons qualify.
    /// Every other case offers the color dot instead.
    #[wasm_bindgen(js_name = "supportsSolidColor")]
    pub fn supports_solid_color(&self) -> bool {
        self.capabilities().solid_color
    }

    /// Which layers the resolved base can realize.
    ///
    /// Prefer this over the individual `supports*` flags when driving UI: it is
    /// one value, and it comes straight from the renderer's own rules rather
    /// than being reassembled on the JS side.
    #[wasm_bindgen(js_name = "capabilities")]
    pub fn capabilities(&self) -> IconCapabilities {
        self.base_kind().capabilities()
    }

    /// Why each unrealizable layer is unavailable, for explaining disabled controls.
    #[wasm_bindgen(js_name = "layerRejections")]
    pub fn layer_rejections(&self) -> LayerRejections {
        self.base_kind().rejections()
    }

    /// Returns `true` if the active customizer uses the vector pipeline.
    #[wasm_bindgen(js_name = "isSvg")]
    pub fn is_svg(&self) -> bool {
        self.base_kind() == IconBaseKind::VectorFolder
    }

    /// Returns `true` if the base icon came from the user rather than the system.
    #[wasm_bindgen(js_name = "isCustom")]
    pub fn is_custom(&self) -> bool {
        self.base_kind() == IconBaseKind::CustomImage
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
        if let ActiveCustomizer::Folder(c) = &mut self.active {
            c.layers
                .solid_color
                .set_config(Some(SolidColorConfig::new(target_r, target_g, target_b)));
            c.layers.solid_color.set_enabled(true);
        }
    }

    /// Sets the solid color enabled state without changing the parameters.
    #[wasm_bindgen(js_name = "setSolidColorEnabled")]
    pub fn set_solid_color_enabled(&mut self, enabled: bool) {
        if let ActiveCustomizer::Folder(c) = &mut self.active {
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
        match &mut self.active {
            ActiveCustomizer::Folder(c) => {
                c.layers.color_dot.set_config(Some(config));
                c.layers.color_dot.set_enabled(true);
            }
            ActiveCustomizer::SvgFolder(c) => {
                c.layers.color_dot.set_config(Some(config));
                c.layers.color_dot.set_enabled(true);
            }
            ActiveCustomizer::Custom(c) => {
                c.layers.color_dot.set_config(Some(config));
                c.layers.color_dot.set_enabled(true);
            }
        }
    }

    /// Sets the color dot enabled state without changing the parameters.
    #[wasm_bindgen(js_name = "setColorDotEnabled")]
    pub fn set_color_dot_enabled(&mut self, enabled: bool) {
        match &mut self.active {
            ActiveCustomizer::Folder(c) => c.layers.color_dot.set_enabled(enabled),
            ActiveCustomizer::SvgFolder(c) => c.layers.color_dot.set_enabled(enabled),
            ActiveCustomizer::Custom(c) => c.layers.color_dot.set_enabled(enabled),
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
        let ActiveCustomizer::Folder(c) = &mut self.active else {
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
        if let ActiveCustomizer::Folder(c) = &mut self.active {
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
        match &mut self.active {
            ActiveCustomizer::Folder(c) => c.layers.overlay.set_enabled(enabled),
            ActiveCustomizer::SvgFolder(c) => c.layers.overlay.set_enabled(enabled),
            ActiveCustomizer::Custom(c) => c.layers.overlay.set_enabled(enabled),
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
        let (width, height) = match &self.active {
            ActiveCustomizer::Folder(c) => Self::base_dimensions(c.base_icons(), size)?,
            ActiveCustomizer::Custom(c) => Self::base_dimensions(c.base_icons(), size)?,
            // Vector icons are resolution-independent, so any size is valid.
            ActiveCustomizer::SvgFolder(_) => (size, size),
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
        match &mut self.active {
            ActiveCustomizer::Folder(_) | ActiveCustomizer::Custom(_) => Ok(None),
            ActiveCustomizer::SvgFolder(c) => c
                .render_output()
                .map(Some)
                .map_err(|e| JsError::new(&e.to_string())),
        }
    }

    // ---- Profile Import/Export ----

    /// Exports the current settings as a JSON string.
    #[wasm_bindgen(js_name = "exportProfileJson")]
    pub fn export_profile_json(&self) -> Result<String, JsError> {
        let profile = match &self.active {
            ActiveCustomizer::Folder(c) => c.export_profile(),
            ActiveCustomizer::SvgFolder(c) => c.export_profile(),
            ActiveCustomizer::Custom(c) => c.export_profile(),
        };
        profile
            .to_json()
            .map_err(|e| JsError::new(&format!("Failed to serialize profile: {}", e)))
    }

    /// Imports settings from a JSON string.
    ///
    /// Intents the active customizer can't realize are dropped, so a profile
    /// saved from one icon still applies to another.
    #[wasm_bindgen(js_name = "importProfileJson")]
    pub fn import_profile_json(&mut self, json: &str) -> Result<(), JsError> {
        let profile = CustomizationProfile::from_json(json)
            .map_err(|e| JsError::new(&format!("Failed to parse profile: {}", e)))?;
        match &mut self.active {
            ActiveCustomizer::Folder(c) => c.apply_profile(&profile),
            ActiveCustomizer::SvgFolder(c) => c.apply_profile(&profile),
            ActiveCustomizer::Custom(c) => c.apply_profile(&profile),
        }
        Ok(())
    }

    /// Clears all customizations and returns to the base icon.
    pub fn reset(&mut self) {
        match &mut self.active {
            ActiveCustomizer::Folder(c) => {
                c.layers.solid_color.set_config(None);
                c.layers.color_dot.set_config(None);
                c.layers.decal.set_config(None);
                c.layers.overlay.set_config(None);
            }
            ActiveCustomizer::SvgFolder(c) => {
                c.layers.color_dot.set_config(None);
                c.layers.overlay.set_config(None);
            }
            ActiveCustomizer::Custom(c) => {
                c.layers.color_dot.set_config(None);
                c.layers.overlay.set_config(None);
            }
        }
    }

    /// Clears the render cache to free memory.
    #[wasm_bindgen(js_name = "clearCache")]
    pub fn clear_cache(&mut self) {
        match &mut self.active {
            ActiveCustomizer::Folder(c) => c.clear_cache(),
            ActiveCustomizer::SvgFolder(c) => c.clear_cache(),
            ActiveCustomizer::Custom(c) => c.clear_cache(),
        }
    }
}
