//! Layer infrastructure for the SVG (vector) rendering medium.
//!
//! Mirrors the raster [`Layer`](crate::layer::Layer) skeleton — optional config,
//! a live-toggle `enabled` flag, and a version counter — but without the
//! per-size raster cache. SVG layers emit markup fragments into an
//! [`SvgCanvas`](crate::medium::SvgCanvas), which is resolution-independent, so
//! there is nothing size-keyed to cache.
//!
//! Layer configs are shared with the raster medium (see
//! [`crate::layer`]); only the rendering logic differs, and it lives on the
//! concrete `SvgLayer<Config>` types.

mod color_dot;
mod overlay;

use crate::layer::LayerConfig;

/// A generic SVG-medium layer with configuration, live toggle, and versioning.
///
/// A layer is **active** when it has a configuration set AND is enabled. The
/// `enabled` flag is for live editing (UI toggles): it is never written to a
/// profile, but it does decide whether the config is, since a profile records
/// what renders rather than what the editor remembers.
#[derive(Debug)]
pub struct SvgLayer<C: LayerConfig> {
    config: Option<C>,
    enabled: bool,
    version: u64,
}

impl<C: LayerConfig> Default for SvgLayer<C> {
    fn default() -> Self {
        Self {
            config: None,
            enabled: true,
            version: 0,
        }
    }
}

impl<C: LayerConfig> SvgLayer<C> {
    /// Returns the current configuration, if any.
    pub fn config(&self) -> Option<&C> {
        self.config.as_ref()
    }

    /// Returns true if the layer has a configuration set.
    pub fn has_config(&self) -> bool {
        self.config.is_some()
    }

    /// Returns the configuration only while the layer is contributing.
    ///
    /// This is what profiles serialize: a configured-but-disabled layer renders
    /// nothing, so it must be indistinguishable from an absent one.
    pub fn active_config(&self) -> Option<&C> {
        if self.enabled {
            self.config.as_ref()
        } else {
            None
        }
    }

    /// Returns true if this layer is active (has config AND is enabled).
    pub fn is_active(&self) -> bool {
        self.enabled && self.config.is_some()
    }

    /// Returns whether the layer is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets whether the layer is enabled. Returns true if the state changed.
    ///
    /// Toggling preserves the configuration and only bumps the version.
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
    /// Increments the version if the config differs.
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

    /// Replaces the configuration and enables the layer exactly when one is given.
    ///
    /// Use this when a profile is the source of truth: the profile must fully
    /// determine what renders, including re-enabling a layer that a UI toggle
    /// had switched off.
    pub fn apply_config(&mut self, config: Option<C>) {
        let enabled = config.is_some();
        self.set_config(config);
        self.set_enabled(enabled);
    }

    /// Bumps the version (e.g. when an upstream dependency changes).
    pub fn invalidate(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
}
