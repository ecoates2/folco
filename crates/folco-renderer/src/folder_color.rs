//! Named folder color presets.
//!
//! The [`FolderColor`] enum and [`FolderColorMetadata`] are defined in
//! `folco-model` (pure data). This module re-exports them and adds
//! [`FolderColorExt`], which bridges a `FolderColor` to the renderer's
//! color layer configs.
//!
//! ```
//! use folco_renderer::folder_color::{FolderColor, FolderColorExt};
//!
//! let color = FolderColor::Red;
//! let solid = color.to_solid_color_config();
//! let dot = color.to_color_dot_config();
//! // Either can be embedded in a CustomizationProfile
//! ```

pub use folco_model::folder_color::{FolderColor, FolderColorMetadata};

use crate::layer::{ColorDotConfig, SolidColorConfig};

/// Bridges the pure [`FolderColor`] model type to the renderer's color configs.
pub trait FolderColorExt {
    /// Converts this preset to a solid-recolor config containing the **target
    /// RGB** color, ready to embed in a
    /// [`CustomizationProfile`](crate::CustomizationProfile). The renderer
    /// computes the necessary deltas from the base icon's surface color.
    fn to_solid_color_config(&self) -> SolidColorConfig;

    /// Converts this preset to a color-dot config.
    fn to_color_dot_config(&self) -> ColorDotConfig;
}

impl FolderColorExt for FolderColor {
    fn to_solid_color_config(&self) -> SolidColorConfig {
        let (r, g, b) = self.rgb();
        SolidColorConfig::new(r, g, b)
    }

    fn to_color_dot_config(&self) -> ColorDotConfig {
        let (r, g, b) = self.rgb();
        ColorDotConfig::new(r, g, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_solid_color_config() {
        let config = FolderColor::Red.to_solid_color_config();
        assert_eq!(config.target_r, 244);
        assert_eq!(config.target_g, 67);
        assert_eq!(config.target_b, 54);
    }

    #[test]
    fn to_color_dot_config() {
        assert_eq!(
            FolderColor::Red.to_color_dot_config(),
            ColorDotConfig::new(244, 67, 54)
        );
    }
}
