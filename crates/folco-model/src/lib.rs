//! folco-model: shared domain value types.
//!
//! These are the pure, serializable value types that every other folco crate
//! builds on: geometry primitives, size specifications and surface color.
//! They deliberately contain **no** logic tied to rendering, platform APIs,
//! Tauri or wasm — only the data and trivial constructors/accessors.
//!
//! Serialization concerns are opt-in via features:
//! - `jsonschema` derives [`schemars::JsonSchema`].
//! - `tsify` derives [`tsify::Tsify`] and the wasm ABI conversions.

pub mod folder_color;

pub use folder_color::{FolderColor, FolderColorMetadata};

use serde::{Deserialize, Serialize};

// ============================================================================
// Geometry Primitives
// ============================================================================

/// A rectangle defined in pixel coordinates.
///
/// Used to specify regions within an image, such as content bounds
/// that indicate where the actual icon content exists (excluding padding/margins).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct RectPx {
    /// X offset from the left edge of the image
    pub x: u32,
    /// Y offset from the top edge of the image
    pub y: u32,
    /// Width of the rectangle
    pub width: u32,
    /// Height of the rectangle
    pub height: u32,
}

impl RectPx {
    /// Creates a new rectangle with the given position and dimensions.
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Creates a rectangle starting at origin (0, 0) with the given dimensions.
    pub fn from_size(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    /// Returns the right edge coordinate (x + width).
    pub fn right(&self) -> u32 {
        self.x + self.width
    }

    /// Returns the bottom edge coordinate (y + height).
    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }
}

/// A 2D size in pixel units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SizePx {
    pub width: u32,
    pub height: u32,
}

impl SizePx {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Returns true if width equals height.
    pub fn is_square(&self) -> bool {
        self.width == self.height
    }
}

// ============================================================================
// IconSizeSpec
// ============================================================================

/// Specification for a target icon size.
///
/// Used by platform size specifications to describe the set of sizes
/// an icon should be rasterized to for compatibility with the host OS.
///
/// # Example
///
/// ```
/// use folco_model::IconSizeSpec;
///
/// let spec = IconSizeSpec::new(256, 256, 1.0);
/// assert!(spec.is_square());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
pub struct IconSizeSpec {
    /// Target pixel width.
    pub width: u32,
    /// Target pixel height.
    pub height: u32,
    /// Display scale factor (1.0 for @1x, 2.0 for @2x, etc.).
    pub scale: f32,
}

impl IconSizeSpec {
    /// Creates a new icon size specification.
    pub fn new(width: u32, height: u32, scale: f32) -> Self {
        Self {
            width,
            height,
            scale,
        }
    }

    /// Creates a square icon size specification.
    pub fn square(size: u32, scale: f32) -> Self {
        Self {
            width: size,
            height: size,
            scale,
        }
    }

    /// Returns `true` if this spec is square (width == height).
    pub fn is_square(&self) -> bool {
        self.width == self.height
    }

    /// Returns the maximum dimension.
    pub fn max_dimension(&self) -> u32 {
        self.width.max(self.height)
    }
}

// ============================================================================
// SurfaceColor
// ============================================================================

/// The RGB color of an icon's primary content surface.
///
/// Used as the reference point when computing color target deltas.
/// Each platform defines its own surface color (e.g., RGB(255, 217, 112)
/// for the golden-yellow Windows folder icon).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
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
