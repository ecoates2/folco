//! SVG canvas for the vector rendering pipeline.
//!
//! Vector layers express the same *intents* as raster layers (a folder color,
//! a badge) with vector-friendly techniques such as a color-dot overlay or
//! nested `<image>` elements, appended as markup to an [`SvgCanvas`].

// ============================================================================
// SvgCanvas
// ============================================================================

/// A mutable SVG document under construction.
///
/// Borrows the base folder-icon markup and accumulates overlay fragments that
/// are injected just before the closing `</svg>` tag when serialized. With no
/// overlays the base markup passes through byte-for-byte.
#[derive(Debug, Clone)]
pub struct SvgCanvas<'a> {
    base: &'a str,
    overlays: Vec<String>,
}

impl<'a> SvgCanvas<'a> {
    /// Creates a canvas over base SVG markup.
    pub fn new(base: &'a str) -> Self {
        Self {
            base,
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
    pub fn base(&self) -> &'a str {
        self.base
    }

    /// Serializes the canvas to a single SVG string.
    ///
    /// Overlay fragments are inserted immediately before the final `</svg>`,
    /// or appended if there is none. If no overlays were pushed, the base
    /// markup is returned unchanged.
    pub fn into_svg(self) -> String {
        let split = self.base.rfind("</svg>").unwrap_or(self.base.len());
        let (head, tail) = self.base.split_at(split);
        let overlays_len: usize = self.overlays.iter().map(String::len).sum();

        let mut out = String::with_capacity(self.base.len() + overlays_len);
        out.push_str(head);
        for overlay in &self.overlays {
            out.push_str(overlay);
        }
        out.push_str(tail);
        out
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16"/></svg>"##;

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
