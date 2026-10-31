//! Config + version bookkeeping shared by every layer, in any medium.

use super::LayerConfig;

/// The medium-independent part of a layer: an optional configuration and a
/// version counter for cache invalidation.
///
/// A layer is **active** when it has a configuration set. Presence/absence of
/// config is the sole on/off switch: `Some(config)` = render with this config,
/// `None` = render nothing (base passes through). This makes export/import
/// lossless — what you see is what you get.
#[derive(Debug)]
pub struct LayerState<C: LayerConfig> {
    config: Option<C>,
    version: u64,
}

impl<C: LayerConfig> Default for LayerState<C> {
    fn default() -> Self {
        Self {
            config: None,
            version: 0,
        }
    }
}

impl<C: LayerConfig> LayerState<C> {
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

    /// Bumps the version (e.g. when an upstream dependency changes).
    pub fn invalidate(&mut self) {
        self.version = self.version.wrapping_add(1);
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
        let state: LayerState<Cfg> = LayerState::default();
        assert!(!state.is_active());
        assert_eq!(state.config(), None);
        assert_eq!(state.version(), 0);
    }

    #[test]
    fn set_config_bumps_version_only_on_change() {
        let mut state: LayerState<Cfg> = LayerState::default();

        assert!(state.set_config(Some(Cfg(1))));
        assert_eq!(state.version(), 1);
        assert!(state.is_active());

        // Same config: no change, no bump.
        assert!(!state.set_config(Some(Cfg(1))));
        assert_eq!(state.version(), 1);

        assert!(state.set_config(Some(Cfg(2))));
        assert_eq!(state.version(), 2);

        assert!(state.set_config(None));
        assert_eq!(state.version(), 3);
        assert!(!state.is_active());

        // None -> None is not a change.
        assert!(!state.set_config(None));
        assert_eq!(state.version(), 3);
    }

    #[test]
    fn invalidate_bumps_version_without_touching_config() {
        let mut state: LayerState<Cfg> = LayerState::default();
        state.set_config(Some(Cfg(7)));
        state.invalidate();
        assert_eq!(state.version(), 2);
        assert_eq!(state.config(), Some(&Cfg(7)));
    }
}
