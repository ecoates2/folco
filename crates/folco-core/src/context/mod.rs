//! Customization context for folder icon operations.
//!
//! [`CustomizationContext`] is the main entry point for all folder icon
//! customization operations. It owns the platform's folder [`Customizer`] and
//! the folder settings provider that writes icons to disk.
//!
//! - [`builder`] constructs it (cache, system icon, pipeline choice).
//! - [`operations`] applies and resets folder icons with progress reporting.
//! - [`custom_image`] customizes a user-supplied image instead of the system icon.

mod builder;
mod custom_image;
mod operations;

pub use builder::{AppInfo, CustomizationContextBuilder};

use folco_renderer::{CustomizationProfile, Customizer, FolderIconBase, IconBase, LayerKind};
use icon_sys::folder_settings::PlatformFolderSettingsProvider;

/// A profile layer the active folder icon base cannot realize.
///
/// The platform picks the medium, so a profile that works on one system may
/// carry entries another can't honour. Applying such a profile drops them
/// silently, which is indistinguishable from success — callers that need to
/// surface it should pre-flight with
/// [`CustomizationContext::unsupported_layers`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedLayer {
    /// The layer that can't be realized.
    pub layer: LayerKind,
    /// Why the active base can't realize it, phrased for end users.
    pub reason: &'static str,
}

impl std::fmt::Display for UnsupportedLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.layer.fmt(f)
    }
}

/// Main context for folder icon customization operations.
///
/// This struct provides the primary API for:
/// - Customizing folder icons with profiles
/// - Resetting folders to default icons
/// - Exposing the base folder icon for frontend previews
///
/// # Example
///
/// ```ignore
/// use folco_core::{CustomizationContextBuilder, CustomizationProfile, SolidColorConfig};
/// use folco_core::progress::progress_channel;
/// use std::path::PathBuf;
///
/// // Build the context
/// let mut ctx = CustomizationContextBuilder::new().build()?;
///
/// // Apply a customization profile
/// let profile = CustomizationProfile::new()
///     .with_solid_color(SolidColorConfig::new(33, 150, 243));
///
/// let folders = vec![PathBuf::from("/path/to/folder")];
/// let (tx, _rx) = progress_channel(32);
/// ctx.customize_folders_async(folders.clone(), profile, tx).await;
///
/// // Reset to default
/// let (tx, _rx) = progress_channel(32);
/// ctx.reset_folders_async(folders, tx).await;
/// ```
pub struct CustomizationContext {
    customizer: Customizer,
    folder_provider: PlatformFolderSettingsProvider,
}

impl CustomizationContext {
    /// Returns the base icon data (icons + surface color).
    ///
    /// This is useful for folco-gui to pass icon images and the surface
    /// color to the WASM renderer.
    ///
    /// Returns `None` when the platform provided a vector folder icon — use
    /// [`folder_icon_svg`](Self::folder_icon_svg) in that case.
    pub fn folder_icon_base(&self) -> Option<&FolderIconBase> {
        match &self.customizer {
            Customizer::Raster(c) => match c.base() {
                IconBase::Folder(base) => Some(base),
                IconBase::Custom(_) => unreachable!("the folder customizer has a folder base"),
            },
            Customizer::Vector(_) => None,
        }
    }

    /// Returns the base folder icon as scalable SVG markup, if available.
    ///
    /// Returns `Some` only when the platform provided a vector folder icon.
    /// Frontends should prefer rendering this markup directly for crisp
    /// scaling, falling back to [`folder_icon_base`](Self::folder_icon_base)
    /// when it returns `None`.
    pub fn folder_icon_svg(&self) -> Option<String> {
        self.customizer.base_svg().map(str::to_owned)
    }

    /// Applies a customization profile to the customizer.
    ///
    /// This configures all layers according to the profile settings. Entries
    /// the active medium can't realize are dropped; check
    /// [`unsupported_layers`](Self::unsupported_layers) first if that should be
    /// reported rather than ignored.
    pub fn apply_profile(&mut self, profile: CustomizationProfile) {
        self.customizer.apply_profile(profile);
    }

    /// Returns the profile entries this platform's folder icon medium can't
    /// realize, in the order they appear in the pipeline.
    ///
    /// Empty means the profile applies in full. Non-interactive callers should
    /// treat a non-empty result as an error: applying anyway produces a
    /// stock-looking icon and reports success.
    pub fn unsupported_layers(&self, profile: &CustomizationProfile) -> Vec<UnsupportedLayer> {
        self.customizer
            .base_kind()
            .unsupported_in(profile)
            .into_iter()
            .map(|(layer, reason)| UnsupportedLayer { layer, reason })
            .collect()
    }
}
