//! Rendering medium abstraction.
//!
//! A [`Medium`] ties together the *canvas* a layer pipeline draws onto and the
//! *output* it ultimately produces. Two media exist:
//!
//! - [`RasterMedium`] — pixel-based. Layers mutate an [`IconImage`] and the
//!   pipeline yields an [`IconSet`] of concrete sizes. This is the classic
//!   folder/custom pipeline (HSL color targeting, tile compositing, etc.).
//! - [`SvgMedium`] — vector-based. Layers append markup to an [`SvgCanvas`]
//!   and the pipeline yields a single scalable SVG [`String`]. Used when the
//!   platform hands us a scalable folder icon (e.g. GNOME icon themes).
//!
//! Splitting by medium lets each layer strategy stay honest about what it can
//! do: raster layers can HSL-shift pixels, while SVG layers must express the
//! same *intent* (a folder color, a badge) with vector-friendly techniques
//! such as a color-dot overlay or nested `<image>` elements.
//!
//! # Status
//!
//! The raster pipeline does not yet consume [`RasterMedium`] directly; it is
//! defined here as the documented contract that the raster customizer will
//! adopt in a later phase. The SVG pipeline is built on [`SvgMedium`] /
//! [`SvgCanvas`] as of Phase 1.

use crate::icon::{IconImage, IconSet};

// ============================================================================
// Medium
// ============================================================================

/// A rendering medium: the canvas layers draw onto plus the final output type.
pub trait Medium {
    /// The mutable drawing surface threaded through the layer pipeline.
    type Canvas;

    /// The final rendered artifact produced from the canvas.
    type Output;
}

/// Pixel-based rendering medium (the classic raster pipeline).
///
/// Uninhabited marker type — it exists to name the raster canvas/output pair,
/// not to be instantiated.
#[derive(Debug, Clone, Copy)]
pub enum RasterMedium {}

impl Medium for RasterMedium {
    type Canvas = IconImage;
    type Output = IconSet;
}

/// Vector-based rendering medium.
///
/// Layers append markup fragments to an [`SvgCanvas`]; the pipeline serializes
/// the result to a single scalable SVG string.
#[derive(Debug, Clone, Copy)]
pub enum SvgMedium {}

impl Medium for SvgMedium {
    type Canvas = SvgCanvas;
    type Output = String;
}

// ============================================================================
// SvgCanvas
// ============================================================================

/// A mutable SVG document under construction.
///
/// Wraps the base folder-icon markup and accumulates overlay fragments that
/// are injected just before the closing `</svg>` tag when serialized. With no
/// overlays the base markup passes through byte-for-byte.
#[derive(Debug, Clone)]
pub struct SvgCanvas {
    base: String,
    overlays: Vec<String>,
}

impl SvgCanvas {
    /// Creates a canvas from base SVG markup.
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            overlays: Vec::new(),
        }
    }

    /// Appends an SVG fragment to be injected before the closing `</svg>`.
    ///
    /// Fragments are emitted in insertion order, so later overlays paint on
    /// top of earlier ones.
    pub fn push_overlay(&mut self, fragment: impl Into<String>) {
        self.overlays.push(fragment.into());
    }

    /// Returns the base markup without any overlays applied.
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Serializes the canvas to a single SVG string.
    ///
    /// Overlay fragments are inserted immediately before the final `</svg>`.
    /// If no overlays were pushed, the base markup is returned unchanged.
    pub fn into_svg(self) -> String {
        if self.overlays.is_empty() {
            return self.base;
        }

        let fragments = self.overlays.concat();
        match self.base.rfind("</svg>") {
            Some(idx) => {
                let mut out = String::with_capacity(self.base.len() + fragments.len());
                out.push_str(&self.base[..idx]);
                out.push_str(&fragments);
                out.push_str(&self.base[idx..]);
                out
            }
            None => {
                let mut out = self.base;
                out.push_str(&fragments);
                out
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str =
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16"/></svg>"##;

    #[test]
    fn passthrough_returns_base_unchanged() {
        let canvas = SvgCanvas::new(BASE);
        assert_eq!(canvas.into_svg(), BASE);
    }

    #[test]
    fn overlays_inject_before_closing_tag() {
        let mut canvas = SvgCanvas::new(BASE);
        canvas.push_overlay(r##"<circle cx="8" cy="8" r="2"/>"##);
        let out = canvas.into_svg();

        assert!(out.ends_with("</svg>"));
        assert!(out.contains(r##"<circle cx="8" cy="8" r="2"/></svg>"##));
        // The base content still precedes the overlay.
        let rect_idx = out.find("<rect").unwrap();
        let circle_idx = out.find("<circle").unwrap();
        assert!(rect_idx < circle_idx);
    }

    #[test]
    fn overlays_preserve_insertion_order() {
        let mut canvas = SvgCanvas::new(BASE);
        canvas.push_overlay("<first/>");
        canvas.push_overlay("<second/>");
        let out = canvas.into_svg();
        assert!(out.find("<first/>").unwrap() < out.find("<second/>").unwrap());
    }

    #[test]
    fn missing_closing_tag_appends_fragments() {
        let mut canvas = SvgCanvas::new("<svg>broken");
        canvas.push_overlay("<extra/>");
        assert_eq!(canvas.into_svg(), "<svg>broken<extra/>");
    }
}
