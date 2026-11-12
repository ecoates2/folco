//! Layer infrastructure for icon customization.
//!
//! A layer is a [`Layer<C>`]: an optional configuration `C` plus a version
//! counter. `Some(config)` means the layer is active and will render; `None`
//! means it renders nothing. Presence of config is the sole on/off switch, so
//! export/import is lossless.
//!
//! Rendering is the config's job. A raster config implements [`RasterPaint`]
//! to draw itself into a [`RenderContext`]; the blanket [`Stage`] impl on
//! `Layer<C>` then skips inactive layers. Layer sets list their layers as
//! stages, **in execution order**, and the engine caches the context after
//! each stage — so changing a later layer never re-runs an earlier one.
//!
//! The vector medium reuses `Layer<C>` and the configs, with its own paint
//! trait: see [`crate::svg_layer`].

pub mod color_dot;
pub mod decal;
pub mod image_source;
pub mod overlay;
pub mod solid_color;
pub mod svg;

pub use color_dot::ColorDotConfig;
pub use decal::DecalConfig;
pub use image_source::ImageSource;
pub use overlay::{ImageOverlayConfig, OverlayAnchorMode, OverlayPosition};
pub use solid_color::SolidColorConfig;
pub use svg::SvgSource;

use crate::error::RenderError;
use crate::icon::{IconImage, SurfaceColor};

// ============================================================================
// Render Context
// ============================================================================

/// The image being built, plus what upstream stages have learned about it.
///
/// Flows through the raster pipeline in stage order. The engine snapshots it
/// after every stage, so everything a later stage needs must live here.
#[derive(Debug, Clone)]
pub struct RenderContext {
    /// The current image being processed through the pipeline.
    pub image: IconImage,

    /// The base icon's reference color, when the base is a system folder icon.
    ///
    /// Set by the engine before any stage runs. Surface-relative layers
    /// (solid color, decal) need it; draw-on-top layers ignore it.
    pub surface_color: Option<SurfaceColor>,

    /// The color the icon's face currently reads as, if a stage has changed it.
    ///
    /// Emitted by solid color so the decal tints against the recolored face
    /// rather than the original surface.
    pub dominant_color: Option<DominantColor>,
}

impl RenderContext {
    /// Creates a context over `image` with no surface color.
    pub fn new(image: IconImage) -> Self {
        Self {
            image,
            surface_color: None,
            dominant_color: None,
        }
    }
}

// ============================================================================
// Common Properties
// ============================================================================

/// The dominant color sampled from the image.
///
/// Emitted by layers that modify the image appearance (like solid color).
/// Consumed by layers that need to derive colors from the image (like decal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DominantColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl DominantColor {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn as_tuple(&self) -> (u8, u8, u8, u8) {
        (self.r, self.g, self.b, self.a)
    }
}

// ============================================================================
// Layer Traits
// ============================================================================

/// Trait for layer configuration types.
///
/// Configurations are pure data — they hold only the user's settings
/// and know how to detect meaningful changes for cache invalidation.
pub trait LayerConfig: Clone {
    /// Returns true if this config differs from another in a way that
    /// would produce different rendering output.
    fn differs_from(&self, other: &Self) -> bool;
}

/// How a raster layer config draws itself.
///
/// Implemented on the config type, not the layer: the layer only decides
/// *whether* to paint (see [`Stage`]); the config decides *what*.
pub trait RasterPaint {
    /// Draws this config into `ctx`, mutating `ctx.image` in place.
    ///
    /// May also update `ctx` properties for downstream stages.
    fn paint(&self, ctx: &mut RenderContext) -> Result<(), RenderError>;
}

/// One step of the raster pipeline, as the engine sees it.
///
/// Object-safe so a layer set can hand back its layers as a plain list.
pub trait Stage {
    /// Changes whenever this stage's output could change.
    fn version(&self) -> u64;

    /// Runs the stage against `ctx`.
    fn run(&self, ctx: &mut RenderContext) -> Result<(), RenderError>;
}

impl<C: LayerConfig + RasterPaint> Stage for Layer<C> {
    fn version(&self) -> u64 {
        Layer::version(self)
    }

    fn run(&self, ctx: &mut RenderContext) -> Result<(), RenderError> {
        match self.config() {
            Some(config) => config.paint(ctx),
            None => Ok(()),
        }
    }
}

// ============================================================================
// Layer
// ============================================================================

/// An optional configuration and a version counter for cache invalidation.
///
/// A layer is **active** when it has a configuration set. Presence/absence of
/// config is the sole on/off switch: `Some(config)` = render with this config,
/// `None` = render nothing (base passes through). This makes export/import
/// lossless — what you see is what you get.
#[derive(Debug)]
pub struct Layer<C: LayerConfig> {
    config: Option<C>,
    version: u64,
}

impl<C: LayerConfig> Default for Layer<C> {
    fn default() -> Self {
        Self {
            config: None,
            version: 0,
        }
    }
}

impl<C: LayerConfig> Layer<C> {
    /// Returns the current configuration, if any.
    ///
    /// `Some(config)` means the layer is active and will render.
    /// `None` means the layer is inactive and produces no output.
    pub fn config(&self) -> Option<&C> {
        self.config.as_ref()
    }

    /// Returns true if this layer is active (has a configuration set).
    pub fn is_active(&self) -> bool {
        self.config.is_some()
    }

    /// Returns the current version number.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Sets the configuration. Returns true if it changed.
    ///
    /// Setting to `Some(config)` activates the layer; setting to `None`
    /// deactivates it. Increments the version when the config differs.
    pub fn set_config(&mut self, config: Option<C>) -> bool {
        let differs = match (&self.config, &config) {
            (None, None) => false,
            (Some(_), None) | (None, Some(_)) => true,
            (Some(old), Some(new)) => old.differs_from(new),
        };

        if differs {
            self.config = config;
            self.version = self.version.wrapping_add(1);
            true
        } else {
            false
        }
    }
}

// ============================================================================
// CacheKey
// ============================================================================

/// Identifies one base image size in a cache.
///
/// Uses width, height, and scale (as integer bits) to identify unique image sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    width: u32,
    height: u32,
    scale_bits: u32,
}

impl CacheKey {
    /// Creates a cache key for the given dimensions and scale.
    pub fn new(width: u32, height: u32, scale: f32) -> Self {
        Self {
            width,
            height,
            scale_bits: scale.to_bits(),
        }
    }

    /// Creates a cache key from an icon image.
    pub fn from_icon(icon: &IconImage) -> Self {
        Self::new(icon.data.width(), icon.data.height(), icon.scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Cfg(u8);

    impl LayerConfig for Cfg {
        fn differs_from(&self, other: &Self) -> bool {
            self != other
        }
    }

    #[test]
    fn starts_inactive_at_version_zero() {
        let layer: Layer<Cfg> = Layer::default();
        assert!(!layer.is_active());
        assert_eq!(layer.config(), None);
        assert_eq!(layer.version(), 0);
    }

    #[test]
    fn set_config_bumps_version_only_on_change() {
        let mut layer: Layer<Cfg> = Layer::default();

        assert!(layer.set_config(Some(Cfg(1))));
        assert_eq!(layer.version(), 1);
        assert!(layer.is_active());

        // Same config: no change, no bump.
        assert!(!layer.set_config(Some(Cfg(1))));
        assert_eq!(layer.version(), 1);

        assert!(layer.set_config(Some(Cfg(2))));
        assert_eq!(layer.version(), 2);

        assert!(layer.set_config(None));
        assert_eq!(layer.version(), 3);
        assert!(!layer.is_active());

        // None -> None is not a change.
        assert!(!layer.set_config(None));
        assert_eq!(layer.version(), 3);
    }
}
