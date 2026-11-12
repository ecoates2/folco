//! Construction of a [`CustomizationContext`]: cache, system icon, pipeline.

use std::path::PathBuf;

use folco_renderer::{Customizer, RasterCustomizer, SvgCustomizer};
use icon_sys::IconSet as SysIconSet;
use icon_sys::folder_settings::{FolderSettingsProvider, PlatformFolderSettingsProvider};

use super::CustomizationContext;
use crate::cache::{CacheConfig, IconCache};
use crate::error::Result;
use crate::sys::RasterFolderMetadata;
use crate::{raster_base, svg_base};

/// Application identification for determining data directories.
///
/// Used with the `directories` crate to locate platform-appropriate
/// app data directories.
#[derive(Debug, Clone)]
pub struct AppInfo {
    /// Reverse domain qualifier (e.g., "com")
    pub qualifier: String,
    /// Organization name (e.g., "ecoates2")
    pub organization: String,
    /// Application name (e.g., "folco")
    pub application: String,
}

impl AppInfo {
    /// Creates a new `AppInfo` with the given values.
    pub fn new(
        qualifier: impl Into<String>,
        organization: impl Into<String>,
        application: impl Into<String>,
    ) -> Self {
        Self {
            qualifier: qualifier.into(),
            organization: organization.into(),
            application: application.into(),
        }
    }
}

impl Default for AppInfo {
    /// Returns the default app info for folco: `com.ecoates2.folco`
    fn default() -> Self {
        Self {
            qualifier: "com".to_string(),
            organization: "ecoates2".to_string(),
            application: "folco".to_string(),
        }
    }
}

/// Builder for creating a [`CustomizationContext`].
///
/// # Example
///
/// ```ignore
/// use folco_core::{AppInfo, CustomizationContextBuilder};
///
/// // Use default app info (com.ecoates2.folco)
/// let ctx = CustomizationContextBuilder::new().build()?;
///
/// // Or with custom app info
/// let ctx = CustomizationContextBuilder::new()
///     .with_app_info(AppInfo::new("com", "example", "myapp"))
///     .build()?;
/// ```
pub struct CustomizationContextBuilder {
    app_info: AppInfo,
    cache_dir: Option<PathBuf>,
    force_cache_refresh: bool,
}

impl CustomizationContextBuilder {
    /// Creates a new builder with default settings.
    ///
    /// Uses the default app info (`com.ecoates2.folco`) for the cache directory.
    pub fn new() -> Self {
        Self {
            app_info: AppInfo::default(),
            cache_dir: None,
            force_cache_refresh: false,
        }
    }

    /// Sets custom application info for determining the cache directory.
    ///
    /// This uses the `directories` crate to find the appropriate
    /// app data directory for the current platform.
    ///
    /// By default, uses `com.ecoates2.folco`.
    pub fn with_app_info(mut self, app_info: AppInfo) -> Self {
        self.app_info = app_info;
        self
    }

    /// Sets a custom cache directory.
    ///
    /// This overrides the app info if both are set.
    pub fn with_cache_dir(mut self, cache_dir: impl Into<PathBuf>) -> Self {
        self.cache_dir = Some(cache_dir.into());
        self
    }

    /// Forces the cache to be refreshed on build.
    pub fn with_force_cache_refresh(mut self, force: bool) -> Self {
        self.force_cache_refresh = force;
        self
    }

    /// Builds the [`CustomizationContext`].
    ///
    /// This will:
    /// 1. Set up the icon cache
    /// 2. Load or fetch the default system folder icon
    /// 3. Choose the customization pipeline for the icon's medium
    /// 4. Initialize the folder settings provider
    pub fn build(self) -> Result<CustomizationContext> {
        let cache_config = if let Some(cache_dir) = self.cache_dir {
            CacheConfig::new(cache_dir).with_force_refresh(self.force_cache_refresh)
        } else {
            CacheConfig::from_app_info(
                &self.app_info.qualifier,
                &self.app_info.organization,
                &self.app_info.application,
            )?
            .with_force_refresh(self.force_cache_refresh)
        };

        let cache = IconCache::new(cache_config);
        let sys_set = cache.get_sys_icon_set()?;

        Ok(CustomizationContext {
            customizer: folder_customizer(sys_set)?,
            folder_provider: PlatformFolderSettingsProvider::new(),
        })
    }
}

impl Default for CustomizationContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds the folder customizer for this platform from a system icon set.
///
/// This is the single place that decides the icon medium: platforms with
/// raster metadata for their stock folder icon (Windows) use the raster
/// pipeline; every other platform must supply a scalable SVG and uses the
/// vector pipeline, disregarding raster images. The renderer stays
/// platform-agnostic.
///
/// # Errors
///
/// Returns [`Error::MissingScalableFolderIcon`](crate::Error::MissingScalableFolderIcon)
/// on vector-only platforms if the set has no SVG variant.
fn folder_customizer(sys_set: SysIconSet) -> Result<Customizer> {
    match RasterFolderMetadata::current_platform() {
        Some(metadata) => {
            let base = raster_base::from_sys_icon_set(sys_set, &metadata);
            Ok(RasterCustomizer::from_folder(base).into())
        }
        None => {
            let base = svg_base::from_sys_icon_set(sys_set)?;
            Ok(SvgCustomizer::from_folder(base).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_has_default_app_info() {
        let builder = CustomizationContextBuilder::new();
        assert_eq!(builder.app_info.qualifier, "com");
        assert_eq!(builder.app_info.organization, "ecoates2");
        assert_eq!(builder.app_info.application, "folco");
    }

    #[test]
    fn builder_with_cache_dir() {
        // Only inspects builder state; never fetches system icons.
        let builder = CustomizationContextBuilder::new()
            .with_cache_dir("/tmp/test_folco")
            .with_force_cache_refresh(true);

        assert!(builder.cache_dir.is_some());
        assert!(builder.force_cache_refresh);
    }

    #[test]
    fn builder_with_custom_app_info() {
        let builder = CustomizationContextBuilder::new()
            .with_app_info(AppInfo::new("org", "example", "myapp"));

        assert_eq!(builder.app_info.qualifier, "org");
        assert_eq!(builder.app_info.organization, "example");
        assert_eq!(builder.app_info.application, "myapp");
    }

    #[test]
    fn app_info_default() {
        let info = AppInfo::default();
        assert_eq!(info.qualifier, "com");
        assert_eq!(info.organization, "ecoates2");
        assert_eq!(info.application, "folco");
    }
}
