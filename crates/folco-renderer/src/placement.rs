//! Default placement for layers drawn on top of the base icon.
//!
//! Defaults depend on the base's medium, not the platform: the platform only
//! decides which medium a base arrives in.

use serde::Serialize;

use crate::capabilities::IconBaseKind;
use crate::layer::OverlayPosition;

/// Where a top layer sits on the base, and its size relative to the base's content.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Placement {
    pub position: OverlayPosition,
    pub scale: f32,
}

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
            scale: 0.7,
        },
        color_dot: Placement {
            position: OverlayPosition::Center,
            scale: 0.7,
        },
    };

    /// Over a vector base: a badge in the bottom-right corner.
    pub const VECTOR: Self = Self {
        overlay: Placement {
            position: OverlayPosition::BottomRight,
            scale: 0.5,
        },
        color_dot: Placement {
            position: OverlayPosition::BottomRight,
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
