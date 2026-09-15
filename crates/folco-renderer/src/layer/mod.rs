//! Layer infrastructure for icon customization.
//!
//! This module provides the generic layer system used by `FolderIconCustomizer`.
//! Each layer encapsulates an optional configuration, version tracking
//! for cache invalidation, and a per-size output cache.
//!
//! # Architecture
//!
//! Each layer config implements [`LayerConfig`] (pure data with change
//! detection). Rendering logic lives on the concrete `Layer<Config>` types.
//!
//! - **Base layers** (e.g., solid color) transform the icon image directly
//!   and cache the full result.
//! - **Stackable layers** (e.g., color dot, decal, overlay) render to a
//!   transparent tile of the same dimensions, which the pipeline composites
//!   on top.
//!
//! Properties flow through the pipeline via [`RenderContext`], enabling
//! layers to communicate without tight coupling.

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

use crate::icon::IconImage;
use image::RgbaImage;
use std::any::{Any, TypeId};
use std::collections::HashMap;

// ============================================================================
// Render Context
// ============================================================================

/// Context that flows through the rendering pipeline.
///
/// Layers can read properties set by upstream layers and emit new properties
/// for downstream layers to consume. This enables loose coupling between layers.
///
/// # Example
///
/// ```ignore
/// // Upstream layer emits a property
/// ctx.set(DominantColor(r, g, b, a));
///
/// // Downstream layer reads the property
/// if let Some(color) = ctx.get::<DominantColor>() {
///     // Use the color...
/// }
/// ```
pub struct RenderContext {
    /// The current image being processed through the pipeline.
    pub image: IconImage,

    /// Typed property bag for inter-layer communication.
    properties: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl RenderContext {
    /// Creates a new render context with the given base image.
    pub fn new(image: IconImage) -> Self {
        Self {
            image,
            properties: HashMap::new(),
        }
    }

    /// Sets a typed property that downstream layers can read.
    pub fn set<T: Any + Send + Sync>(&mut self, value: T) {
        self.properties.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Gets a typed property set by an upstream layer.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.properties
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref())
    }

    /// Checks if a property has been set.
    pub fn has<T: Any + Send + Sync>(&self) -> bool {
        self.properties.contains_key(&TypeId::of::<T>())
    }
}

// ============================================================================
// Common Properties
// ============================================================================

/// The dominant color sampled from the image.
///
/// Emitted by layers that modify the image appearance (like solid color).
/// Consumed by layers that need to derive colors from the image (like decal).
#[derive(Debug, Clone, Copy)]
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
/// Rendering logic lives on the concrete [`Layer`] types.
pub trait LayerConfig: Clone {
    /// Returns true if this config differs from another in a way that
    /// would produce different rendering output.
    fn differs_from(&self, other: &Self) -> bool;
}

// ============================================================================
// Layer Dependencies
// ============================================================================

/// Represents the combined version of upstream layer dependencies.
///
/// This is used to detect when a layer's cache is stale because an
/// upstream layer has changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DependencyVersion(u64);

impl DependencyVersion {
    /// No dependencies (root layer).
    pub const NONE: Self = Self(0);

    /// Creates a dependency version from a single version number.
    pub fn from_version(version: u64) -> Self {
        Self(version)
    }

    /// Combines multiple upstream layer versions into one.
    ///
    /// Order-sensitive on purpose. Summing would let distinct pipeline states
    /// collide — `[1, 0]` and `[0, 1]` sum alike — and a collision hands back a
    /// stale cache entry as though it were fresh.
    pub fn combine(versions: &[u64]) -> Self {
        const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

        let mut hash = FNV_OFFSET;
        for version in versions {
            hash ^= version;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        Self(hash)
    }
}

// ============================================================================
// CacheKey
// ============================================================================

/// Key for cached rendered images.
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

// ============================================================================
// Generic Layer
// ============================================================================

// ============================================================================
// Layer Cache Output
// ============================================================================

/// Cached result from a layer's rendering.
///
/// Layers produce different types of output:
/// - **Image-transforming layers** (e.g., solid_color) modify `ctx.image`
///   directly and cache the full transformed result.
/// - **Tile layers** (e.g., color_dot, decal, overlay) render to a transparent
///   canvas of the same dimensions, which the pipeline composites on top.
#[derive(Debug)]
enum CachedOutput {
    /// Full transformed image (e.g., solid_color mutates the base icon).
    Image(IconImage),
    /// Transparent tile for compositing (e.g., decal, overlay).
    Tile(RgbaImage),
}

// ============================================================================
// Generic Layer
// ============================================================================

/// A generic layer with configuration, caching, and version tracking.
///
/// The layer tracks:
/// - Optional configuration of type `C` — `Some(config)` means active, `None` means inactive
/// - A version number that increments on any state change
/// - A cache of rendered outputs keyed by size
/// - The dependency version when each cache entry was stored
///
/// A layer is **active** when it has a configuration set. Presence/absence of
/// config is the sole on/off switch: `Some(config)` = render with this config,
/// `None` = render nothing (base passes through). This makes export/import
/// lossless — what you see is what you get.
#[derive(Debug)]
pub struct Layer<C: LayerConfig> {
    config: Option<C>,
    version: u64,
    cache: HashMap<CacheKey, (CachedOutput, u64)>,
}

impl<C: LayerConfig> Default for Layer<C> {
    fn default() -> Self {
        Self {
            config: None,
            version: 0,
            cache: HashMap::new(),
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
    /// deactivates it. Clears the cache and increments version when
    /// the config differs.
    pub fn set_config(&mut self, config: Option<C>) -> bool {
        let differs = match (&self.config, &config) {
            (None, None) => false,
            (Some(_), None) | (None, Some(_)) => true,
            (Some(old), Some(new)) => old.differs_from(new),
        };

        if differs {
            self.config = config;
            self.version = self.version.wrapping_add(1);
            self.cache.clear();
            true
        } else {
            false
        }
    }

    /// Invalidates the cache and increments version.
    ///
    /// Called when upstream layers change.
    pub fn invalidate(&mut self) {
        self.version = self.version.wrapping_add(1);
        self.cache.clear();
    }

    /// Gets a cached output if valid for the given key and dependency version.
    fn get_cached(&self, key: CacheKey, deps: DependencyVersion) -> Option<&CachedOutput> {
        self.cache.get(&key).and_then(|(output, stored_dep)| {
            if *stored_dep == deps.0 {
                Some(output)
            } else {
                None
            }
        })
    }

    /// Stores a layer output in the cache with the current dependency version.
    fn store(&mut self, key: CacheKey, output: CachedOutput, deps: DependencyVersion) {
        self.cache.insert(key, (output, deps.0));
    }
}

// NOTE: Rendering methods (apply, render_tile) are implemented on `Layer<SpecificConfig>`
// in each layer module (solid_color.rs, color_dot.rs, decal.rs, overlay.rs).

// ============================================================================
// Composite Layer
// ============================================================================

/// A cache-only layer for final composited images.
///
/// Unlike [`Layer<C>`], this has no configuration or enabled state.
/// It purely caches the final rendered output and tracks a version
/// for invalidation when any upstream layer changes.
#[derive(Default)]
pub struct CompositeLayer {
    version: u64,
    cache: HashMap<CacheKey, (IconImage, u64)>,
}

impl CompositeLayer {
    /// Returns the current version number.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Invalidates the cache and increments version.
    pub fn invalidate(&mut self) {
        self.version = self.version.wrapping_add(1);
        self.cache.clear();
    }

    /// Gets a cached image if valid for the given key and dependency version.
    pub fn get_cached(&self, key: CacheKey, deps: DependencyVersion) -> Option<&IconImage> {
        self.cache.get(&key).and_then(|(img, stored_dep)| {
            if *stored_dep == deps.0 {
                Some(img)
            } else {
                None
            }
        })
    }

    /// Stores an image in the cache with the current dependency version.
    pub fn store(&mut self, key: CacheKey, image: IconImage, deps: DependencyVersion) {
        self.cache.insert(key, (image, deps.0));
    }
}

// NOTE: LayerPipeline has been replaced by the LayerSet trait.
// See customizer.rs for the generic engine and
// folder_customizer.rs / custom_customizer.rs for concrete layer sets.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_distinguishes_which_layer_changed() {
        assert_ne!(
            DependencyVersion::combine(&[1, 0]),
            DependencyVersion::combine(&[0, 1])
        );
    }

    /// A collision here would serve a stale composite as if it were current.
    #[test]
    fn combine_is_injective_over_a_small_version_space() {
        let mut seen = std::collections::HashSet::new();
        for a in 0..12u64 {
            for b in 0..12u64 {
                for c in 0..12u64 {
                    assert!(
                        seen.insert(DependencyVersion::combine(&[a, b, c])),
                        "collision at {a},{b},{c}"
                    );
                }
            }
        }
    }
}
