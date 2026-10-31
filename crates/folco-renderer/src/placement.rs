//! Where a top layer sits on the base icon, and how big it is.
//!
//! [`Placement`] is the value type shared by every layer that draws on top of
//! the base (color dot, overlay). The rule for turning it into coordinates is
//! written once, in [`Placement::origin`], and used by both media: the raster
//! layers call it in pixels, the SVG layers in percent.
//!
//! Defaults depend on the base's medium, not the platform: the platform only
//! decides which medium a base arrives in.

use std::ops::{Add, Neg, Sub};

use serde::{Deserialize, Serialize};

use crate::capabilities::IconBaseKind;
use crate::layer::{OverlayAnchorMode, OverlayPosition};

// ============================================================================
// Placement
// ============================================================================

/// Where a top layer sits on the base, and its size relative to the base's content.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    /// Which corner (or the center) the layer is placed at.
    pub position: OverlayPosition,
    /// Whether the layer is kept inside the content bounds or centered on the corner.
    #[serde(default)]
    pub anchor_mode: OverlayAnchorMode,
    /// Edge length as a fraction of the base's shorter content dimension (0.0-1.0).
    pub scale: f32,
}

impl Placement {
    /// Creates a placement, clamping `scale` to 0.0-1.0.
    pub fn new(position: OverlayPosition, anchor_mode: OverlayAnchorMode, scale: f32) -> Self {
        Self {
            position,
            anchor_mode,
            scale: scale.clamp(0.0, 1.0),
        }
    }

    /// Whether `other` would render differently from `self`.
    ///
    /// The scale is compared with a small epsilon so float noise from a UI
    /// slider doesn't invalidate caches.
    pub fn differs_from(&self, other: &Self) -> bool {
        self.position != other.position
            || self.anchor_mode != other.anchor_mode
            || (self.scale - other.scale).abs() > 0.0001
    }

    /// Top-left origin of a `width`×`height` layer inside a `total_width`×`total_height`
    /// box, relative to the box's own origin.
    ///
    /// Units are the caller's: pixels (`i32`) for the raster pipeline, percent
    /// (`f32`) for SVG. The caller picks the type, and with it the rounding —
    /// `i32` halves by integer division, so raster output stays pixel-exact.
    pub(crate) fn origin<T: PlacementScalar>(
        &self,
        total_width: T,
        total_height: T,
        width: T,
        height: T,
    ) -> (T, T) {
        let (horizontal, vertical) = self.position.aligns();
        (
            axis_origin(horizontal, self.anchor_mode, total_width, width),
            axis_origin(vertical, self.anchor_mode, total_height, height),
        )
    }
}

// ============================================================================
// Placement math
// ============================================================================

/// Arithmetic a medium needs to place an edge along one axis.
pub(crate) trait PlacementScalar:
    Copy + Default + Add<Output = Self> + Sub<Output = Self> + Neg<Output = Self>
{
    /// `self / 2`, rounded the way the medium rounds.
    fn half(self) -> Self;
}

impl PlacementScalar for i32 {
    fn half(self) -> Self {
        self / 2
    }
}

impl PlacementScalar for f32 {
    fn half(self) -> Self {
        self / 2.0
    }
}

/// Where a layer sits along one axis.
#[derive(Debug, Clone, Copy)]
enum Align {
    Start,
    Center,
    End,
}

impl OverlayPosition {
    /// The `(horizontal, vertical)` alignment this position names.
    fn aligns(self) -> (Align, Align) {
        match self {
            Self::TopLeft => (Align::Start, Align::Start),
            Self::TopRight => (Align::End, Align::Start),
            Self::BottomLeft => (Align::Start, Align::End),
            Self::BottomRight => (Align::End, Align::End),
            Self::Center => (Align::Center, Align::Center),
        }
    }
}

/// Start of a `size`-long edge within a `total`-long axis.
///
/// `Inset` keeps the edge within the axis; `Centered` centers it on the chosen
/// end so half can hang off (negative or past-`total` offsets).
fn axis_origin<T: PlacementScalar>(
    align: Align,
    anchor: OverlayAnchorMode,
    total: T,
    size: T,
) -> T {
    match (align, anchor) {
        (Align::Center, _) => (total - size).half(),
        (Align::Start, OverlayAnchorMode::Inset) => T::default(),
        (Align::End, OverlayAnchorMode::Inset) => total - size,
        (Align::Start, OverlayAnchorMode::Centered) => -size.half(),
        (Align::End, OverlayAnchorMode::Centered) => total - size.half(),
    }
}

// ============================================================================
// DefaultPlacements
// ============================================================================

/// Default placements for the overlay and color dot layers.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(rename_all = "camelCase")]
pub struct DefaultPlacements {
    pub overlay: Placement,
    pub color_dot: Placement,
}

impl DefaultPlacements {
    /// Over a raster base: centered on the content.
    pub const RASTER: Self = Self {
        overlay: Placement {
            position: OverlayPosition::Center,
            anchor_mode: OverlayAnchorMode::Inset,
            scale: 0.7,
        },
        color_dot: Placement {
            position: OverlayPosition::Center,
            anchor_mode: OverlayAnchorMode::Inset,
            scale: 0.7,
        },
    };

    /// Over a vector base: a badge in the bottom-right corner.
    pub const VECTOR: Self = Self {
        overlay: Placement {
            position: OverlayPosition::BottomRight,
            anchor_mode: OverlayAnchorMode::Inset,
            scale: 0.5,
        },
        color_dot: Placement {
            position: OverlayPosition::BottomRight,
            anchor_mode: OverlayAnchorMode::Inset,
            scale: 0.5,
        },
    };
}

impl IconBaseKind {
    /// Default overlay and color dot placement over this base.
    pub fn default_placements(self) -> DefaultPlacements {
        if self.is_vector() {
            DefaultPlacements::VECTOR
        } else {
            DefaultPlacements::RASTER
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::RectPx;

    const POSITIONS: [OverlayPosition; 5] = [
        OverlayPosition::TopLeft,
        OverlayPosition::TopRight,
        OverlayPosition::BottomLeft,
        OverlayPosition::BottomRight,
        OverlayPosition::Center,
    ];
    const ANCHORS: [OverlayAnchorMode; 2] = [OverlayAnchorMode::Inset, OverlayAnchorMode::Centered];

    /// The raster formula as it was hand-written before `Placement::origin`
    /// existed. Kept as an oracle so the shared math can't drift from it.
    fn legacy_raster_position(
        position: OverlayPosition,
        anchor_mode: OverlayAnchorMode,
        bounds: &RectPx,
        overlay_width: u32,
        overlay_height: u32,
    ) -> (i32, i32) {
        let bx = bounds.x as i32;
        let by = bounds.y as i32;
        let bw = bounds.width as i32;
        let bh = bounds.height as i32;
        let ow = overlay_width as i32;
        let oh = overlay_height as i32;

        match (position, anchor_mode) {
            (OverlayPosition::TopLeft, OverlayAnchorMode::Inset) => (bx, by),
            (OverlayPosition::TopRight, OverlayAnchorMode::Inset) => (bx + bw - ow, by),
            (OverlayPosition::BottomLeft, OverlayAnchorMode::Inset) => (bx, by + bh - oh),
            (OverlayPosition::BottomRight, OverlayAnchorMode::Inset) => {
                (bx + bw - ow, by + bh - oh)
            }
            (OverlayPosition::TopLeft, OverlayAnchorMode::Centered) => (bx - ow / 2, by - oh / 2),
            (OverlayPosition::TopRight, OverlayAnchorMode::Centered) => {
                (bx + bw - ow / 2, by - oh / 2)
            }
            (OverlayPosition::BottomLeft, OverlayAnchorMode::Centered) => {
                (bx - ow / 2, by + bh - oh / 2)
            }
            (OverlayPosition::BottomRight, OverlayAnchorMode::Centered) => {
                (bx + bw - ow / 2, by + bh - oh / 2)
            }
            (OverlayPosition::Center, _) => (bx + (bw - ow) / 2, by + (bh - oh) / 2),
        }
    }

    #[test]
    fn raster_origin_matches_legacy_formula_for_every_combination() {
        // Odd sizes and offset, non-square bounds exercise the integer rounding.
        let cases = [
            (RectPx::new(0, 0, 64, 64), 25, 25),
            (RectPx::new(10, 20, 100, 80), 20, 10),
            (RectPx::new(3, 7, 41, 59), 13, 17),
            (RectPx::new(0, 0, 16, 16), 0, 0),
        ];

        for (bounds, ow, oh) in cases {
            for position in POSITIONS {
                for anchor in ANCHORS {
                    let placement = Placement::new(position, anchor, 0.5);
                    let (x, y) = placement.origin(
                        bounds.width as i32,
                        bounds.height as i32,
                        ow as i32,
                        oh as i32,
                    );
                    let actual = (bounds.x as i32 + x, bounds.y as i32 + y);
                    let expected = legacy_raster_position(position, anchor, &bounds, ow, oh);
                    assert_eq!(actual, expected, "{position:?}/{anchor:?} in {bounds:?}");
                }
            }
        }
    }

    #[test]
    fn percent_origin_inset_bottom_right_stays_in_bounds() {
        let p = Placement::new(OverlayPosition::BottomRight, OverlayAnchorMode::Inset, 0.25);
        assert_eq!(p.origin(100.0, 100.0, 25.0, 25.0), (75.0, 75.0));
    }

    #[test]
    fn percent_origin_centered_can_hang_off() {
        let p = Placement::new(OverlayPosition::TopLeft, OverlayAnchorMode::Centered, 0.4);
        assert_eq!(p.origin(100.0, 100.0, 40.0, 40.0), (-20.0, -20.0));

        let p = Placement::new(
            OverlayPosition::BottomRight,
            OverlayAnchorMode::Centered,
            0.5,
        );
        assert_eq!(p.origin(100.0, 100.0, 50.0, 50.0), (75.0, 75.0));
    }

    #[test]
    fn new_clamps_scale() {
        assert_eq!(
            Placement::new(OverlayPosition::Center, OverlayAnchorMode::Inset, 1.5).scale,
            1.0
        );
        assert_eq!(
            Placement::new(OverlayPosition::Center, OverlayAnchorMode::Inset, -0.5).scale,
            0.0
        );
    }

    #[test]
    fn differs_from_ignores_float_noise_but_not_real_changes() {
        let a = Placement::new(OverlayPosition::Center, OverlayAnchorMode::Inset, 0.5);
        let noise = Placement { scale: 0.50001, ..a };
        let moved = Placement { position: OverlayPosition::TopLeft, ..a };
        let anchored = Placement { anchor_mode: OverlayAnchorMode::Centered, ..a };
        let resized = Placement { scale: 0.4, ..a };

        assert!(!a.differs_from(&noise));
        assert!(a.differs_from(&moved));
        assert!(a.differs_from(&anchored));
        assert!(a.differs_from(&resized));
    }

    #[test]
    fn anchor_mode_defaults_to_inset_in_json() {
        let p: Placement = serde_json::from_str(r#"{"position":"top-left","scale":0.3}"#).unwrap();
        assert_eq!(p.anchor_mode, OverlayAnchorMode::Inset);
    }
}
