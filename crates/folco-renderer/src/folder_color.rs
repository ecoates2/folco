//! Named folder color presets.
//!
//! The [`FolderColor`] enum and [`FolderColorMetadata`] are defined in
//! `folco-model` (pure data). This module re-exports them and adds
//! [`FolderColorExt`], which bridges a `FolderColor` to the renderer's
//! [`FolderColorTargetConfig`].
//!
//! ```
//! use folco_renderer::folder_color::{FolderColor, FolderColorExt};
//!
//! let color = FolderColor::Red;
//! let config = color.to_folder_color_target_config();
//! // config can be embedded in a CustomizationProfile
//! ```

pub use folco_model::folder_color::{FolderColor, FolderColorMetadata};

use crate::layer::FolderColorTargetConfig;

/// Bridges the pure [`FolderColor`] model type to the renderer's
/// [`FolderColorTargetConfig`].
pub trait FolderColorExt {
    /// Converts this color preset to a color target config containing the
    /// **target RGB** color, ready to embed in a
    /// [`CustomizationProfile`](crate::CustomizationProfile). The renderer
    /// computes the necessary deltas from the base icon's surface color.
    fn to_folder_color_target_config(&self) -> FolderColorTargetConfig;
}

impl FolderColorExt for FolderColor {
    fn to_folder_color_target_config(&self) -> FolderColorTargetConfig {
        let (r, g, b) = self.rgb();
        FolderColorTargetConfig::new(r, g, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_folder_color_target_config() {
        let config = FolderColor::Red.to_folder_color_target_config();
        assert_eq!(config.target_r, 244);
        assert_eq!(config.target_g, 67);
        assert_eq!(config.target_b, 54);
    }
}
