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
//! - **Base layers** (e.g., color target) transform the icon image directly
//!   and cache the full result.
//! - **Stackable layers** (e.g., decal, overlay) render to a transparent tile
//!   of the same dimensions, which the pipeline composites on top.
//!
//! Properties flow through the pipeline via [`RenderContext`], enabling
//! layers to communicate without tight coupling.

pub mod folder_color_target;
pub mod decal;
pub mod image_source;
pub mod overlay;
pub mod svg;

pub use folder_color_target::FolderColorTargetConfig;
pub use decal::DecalConfig;
pub use image_source::ImageSource;
pub use overlay::{ImageOverlayConfig, OverlayAnchorMode, OverlayPosition};
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
/// Emitted by layers that modify the image appearance (like color target).
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DependencyVersion(u64);

impl DependencyVersion {
    /// No dependencies (root layer).
    pub const NONE: Self = Self(0);

    /// Creates a dependency version from a single version number.
    pub fn from_version(version: u64) -> Self {
        Self(version)
    }

    /// Creates a dependency version from a single upstream layer.
    pub fn from_layer<C: LayerConfig>(layer: &Layer<C>) -> Self {
        Self(layer.version())
    }

    /// Combines multiple upstream layer versions into one.
    pub fn combine(versions: &[u64]) -> Self {
        Self(versions.iter().fold(0u64, |acc, v| acc.wrapping_add(*v)))
    }
}

// ============================================================================
// Layer Versions
// ============================================================================

/// Snapshot of all layer versions in the pipeline.
///
/// Passed to [`LayerEffect::dependencies`] so each layer can declare
/// which upstream layers it depends on for cache invalidation.
#[derive(Debug, Clone, Copy)]
pub struct LayerVersions {
    /// Version of the color target layer.
    pub folder_color_target: u64,
    /// Version of the decal layer.
    pub decal: u64,
    /// Version of the overlay layer.
    pub overlay: u64,
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
/// - **Image-transforming layers** (e.g., color_target) modify `ctx.image`
///   directly and cache the full transformed result.
/// - **Tile layers** (e.g., decal, overlay) render to a transparent canvas
///   of the same dimensions, which the pipeline composites on top.
enum CachedOutput {
    /// Full transformed image (e.g., color_target mutates the base icon).
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
/// - Optional configuration of type `C`
/// - An enabled flag for live toggling (does not affect config or cache)
/// - A version number that increments on any state change
/// - A cache of rendered outputs keyed by size
/// - The dependency version when each cache entry was stored
///
/// A layer is considered **active** when it has a configuration set
/// AND is enabled. The `enabled` flag is for live editing (UI toggles)
/// and is not serialized into profiles.
pub struct Layer<C: LayerConfig> {
    config: Option<C>,
    enabled: bool,
    version: u64,
    cache: HashMap<CacheKey, (CachedOutput, u64)>,
}

impl<C: LayerConfig> Default for Layer<C> {
    fn default() -> Self {
        Self {
            config: None,
            enabled: true,
            version: 0,
            cache: HashMap::new(),
        }
    }
}

impl<C: LayerConfig> Layer<C> {
    /// Returns the current configuration, if any.
    pub fn config(&self) -> Option<&C> {
        self.config.as_ref()
    }

    /// Returns true if this layer is active (has config AND is enabled).
    pub fn is_active(&self) -> bool {
        self.enabled && self.config.is_some()
    }

    /// Returns true if the layer has a configuration set.
    pub fn has_config(&self) -> bool {
        self.config.is_some()
    }

    /// Returns whether the layer is enabled.
    ///
    /// This is a live-editing toggle that does not affect the stored
    /// configuration or cached outputs. It is not serialized into profiles.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets whether the layer is enabled.
    ///
    /// Toggling preserves the layer's configuration and cache.
    /// Only the version is bumped so downstream/composite caches
    /// know to re-evaluate.
    ///
    /// Returns true if the enabled state changed.
    pub fn set_enabled(&mut self, enabled: bool) -> bool {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.version = self.version.wrapping_add(1);
            true
        } else {
            false
        }
    }

    /// Returns the current version number.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Sets the configuration. Returns true if it changed.
    ///
    /// Clears the cache and increments version if the config differs.
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
// in each layer module (color_target.rs, decal.rs, overlay.rs).

// ============================================================================
// Composite Layer
// ============================================================================

/// A cache-only layer for final composited images.
///
/// Unlike [`Layer<C>`], this has no configuration or enabled state.
/// It purely caches the final rendered output and tracks a version
/// for invalidation when any upstream layer changes.
pub struct CompositeLayer {
    version: u64,
    cache: HashMap<CacheKey, (IconImage, u64)>,
}

impl Default for CompositeLayer {
    fn default() -> Self {
        Self {
            version: 0,
            cache: HashMap::new(),
        }
    }
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
