//! Layer infrastructure for the SVG (vector) rendering medium.
//!
//! Mirrors the raster [`Layer`](crate::layer::Layer) skeleton — optional config
//! and a version counter — but without the per-size raster cache. SVG layers
//! emit markup fragments into an [`SvgCanvas`](crate::medium::SvgCanvas), which
//! is resolution-independent, so there is nothing size-keyed to cache.
//!
//! Layer configs are shared with the raster medium (see
//! [`crate::layer`]); only the rendering logic differs, and it lives on the
//! concrete `SvgLayer<Config>` types.

mod color_dot;
mod overlay;

use crate::layer::LayerConfig;

/// A generic SVG-medium layer with configuration and versioning.
///
/// A layer is **active** when it has a configuration set (`Some(config)`).
/// Absence of config (`None`) means inactive — the layer produces no output.
#[derive(Debug)]
pub struct SvgLayer<C: LayerConfig> {
    config: Option<C>,
    version: u64,
}

impl<C: LayerConfig> Default for SvgLayer<C> {
    fn default() -> Self {
        Self {
            config: None,
            version: 0,
        }
    }
}

impl<C: LayerConfig> SvgLayer<C> {
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

    /// Returns true if the layer has a configuration set.
    ///
    /// Alias for [`is_active`](Self::is_active) — kept for readability
    /// in contexts where "configured" is the natural term.
    pub fn is_configured(&self) -> bool {
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
    pub fn set(&mut self, config: Option<C>) -> bool {
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

    /// Replaces the layer's configuration.
    ///
    /// Use this when a profile is the source of truth: the profile must fully
    /// determine what renders. The layer is active when a config is given,
    /// inactive when `None` is given.
    pub fn apply(&mut self, config: Option<C>) {
        self.set(config);
    }

    /// Bumps the version (e.g. when an upstream dependency changes).
    pub fn invalidate(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
}
