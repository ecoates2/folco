//! Customization context for folder icon operations.
//!
//! This module provides the main entry point for all folder icon customization
//! operations. It manages the icon customizer, folder settings provider, and
//! icon cache.

use crate::cache::{CacheConfig, IconCache};
use crate::convert::{convert_icon_set, convert_icon_set_to_sys};
use crate::error::{Error, Result};
use crate::progress::{Progress, ProgressSender};

use folco_renderer::ImageSource;
use folco_renderer::{
    CustomIconCustomizer, CustomizationProfile, FolderIconBase, FolderIconCustomizer,
    IconSet as RendererIconSet,
};
use icon_sys::folder_settings::{FolderSettingsProvider, PlatformFolderSettingsProvider};

use std::path::{Path, PathBuf};

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
/// use folco_core::CustomizationContextBuilder;
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
    /// 3. Initialize the icon customizer
    /// 4. Initialize the folder settings provider
    pub fn build(self) -> Result<CustomizationContext> {
        // Determine cache configuration
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

        // Create cache and load icons
        let cache = IconCache::new(cache_config);
        let renderer_icons = cache.get_renderer_icon_set()?;

        // Create the customizer with the platform-specific surface color
        let folder_icon_base = FolderIconBase::new(renderer_icons, crate::sys::SURFACE_COLOR);
        let customizer = FolderIconCustomizer::from_folder(folder_icon_base);

        // Create the folder settings provider
        let folder_provider = PlatformFolderSettingsProvider::new();

        Ok(CustomizationContext {
            cache,
            customizer,
            folder_provider,
        })
    }
}

impl Default for CustomizationContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Main context for folder icon customization operations.
///
/// This struct provides the primary API for:
/// - Customizing folder icons with profiles
/// - Resetting folders to default icons
/// - Accessing the icon customizer for live preview
/// - Managing the icon cache
///
/// # Example
///
/// ```ignore
/// use folco_core::{CustomizationContext, CustomizationContextBuilder, CustomizationProfile};
/// use std::path::PathBuf;
///
/// // Build the context
/// let mut ctx = CustomizationContextBuilder::new()
///     .with_app_info("com", "example", "folco")
///     .build()?;
///
/// // Apply a customization profile
/// let profile = CustomizationProfile::new()
///     .with_hue_rotation(HueRotationSettings { degrees: 180.0, enabled: true });
///
/// let folders = vec![PathBuf::from("/path/to/folder")];
/// ctx.customize_folders(&folders, &profile)?;
///
/// // Reset to default
/// ctx.reset_folders(&folders)?;
/// ```
pub struct CustomizationContext {
    cache: IconCache,
    customizer: FolderIconCustomizer,
    folder_provider: PlatformFolderSettingsProvider,
}

impl CustomizationContext {
    /// Returns a reference to the icon customizer.
    ///
    /// Use this for live preview rendering without applying to folders.
    pub fn customizer(&self) -> &FolderIconCustomizer {
        &self.customizer
    }

    /// Returns a mutable reference to the icon customizer.
    ///
    /// Use this to configure layers and render previews.
    pub fn customizer_mut(&mut self) -> &mut FolderIconCustomizer {
        &mut self.customizer
    }

    /// Returns a reference to the icon cache.
    pub fn cache(&self) -> &IconCache {
        &self.cache
    }

    /// Returns a mutable reference to the icon cache.
    pub fn cache_mut(&mut self) -> &mut IconCache {
        &mut self.cache
    }

    /// Returns the base icon data (icons + surface color).
    ///
    /// This is useful for folco-gui to pass icon images and the surface
    /// color to the WASM renderer.
    pub fn folder_icon_base(&self) -> FolderIconBase {
        FolderIconBase::new(
            self.customizer.base_icons().clone(),
            *self
                .customizer
                .surface_color()
                .expect("CustomizationContext always holds a folder-based customizer"),
        )
    }

    /// Applies a customization profile to the customizer.
    ///
    /// This configures all layers according to the profile settings.
    pub fn apply_profile(&mut self, profile: &CustomizationProfile) {
        self.customizer.apply_profile(profile);
    }

    /// Exports the current customizer settings as a profile.
    pub fn export_profile(&self) -> CustomizationProfile {
        self.customizer.export_profile()
    }

    /// Renders and returns the customized icon set.
    ///
    /// This applies all active customizations and returns the result.
    /// The returned icon set is in `folco-renderer` format.
    pub fn render(&mut self) -> Result<RendererIconSet> {
        Ok(self.customizer.render_all()?)
    }

    /// Customizes the icons for the specified folders.
    ///
    /// This method:
    /// 1. Applies the profile to the customizer
    /// 2. Renders the customized icon set
    /// 3. Converts to system format
    /// 4. Applies to each folder
    ///
    /// # Arguments
    ///
    /// * `folders` - Collection of folder paths to customize
    /// * `profile` - The customization profile to apply
    ///
    /// # Returns
    ///
    /// A vector of results, one for each folder. This allows partial success
    /// where some folders succeed and others fail.
    pub fn customize_folders<P: AsRef<Path>>(
        &mut self,
        folders: &[P],
        profile: &CustomizationProfile,
    ) -> Vec<Result<()>> {
        // Apply the profile
        self.apply_profile(profile);

        // Render the customized icons
        let rendered = match self.render() {
            Ok(icons) => icons,
            Err(e) => return vec![Err(e)],
        };

        // Convert to system format
        let sys_icons = convert_icon_set_to_sys(&rendered);

        // Apply to each folder
        folders
            .iter()
            .map(|folder| {
                self.folder_provider
                    .set_icon_for_folder(folder.as_ref(), &sys_icons)
                    .map_err(|e| {
                        Error::FolderCustomization(folder.as_ref().to_path_buf(), e.to_string())
                    })
            })
            .collect()
    }

    /// Resets the icons for the specified folders to the system default.
    ///
    /// # Arguments
    ///
    /// * `folders` - Collection of folder paths to reset
    ///
    /// # Returns
    ///
    /// A vector of results, one for each folder.
    pub fn reset_folders<P: AsRef<Path>>(&self, folders: &[P]) -> Vec<Result<()>> {
        folders
            .iter()
            .map(|folder| {
                self.folder_provider
                    .reset_icon_for_folder(folder.as_ref())
                    .map_err(|e| Error::FolderReset(folder.as_ref().to_path_buf(), e.to_string()))
            })
            .collect()
    }

    /// Customizes a single folder with the given profile.
    ///
    /// Convenience method for customizing a single folder.
    pub fn customize_folder<P: AsRef<Path>>(
        &mut self,
        folder: P,
        profile: &CustomizationProfile,
    ) -> Result<()> {
        self.customize_folders(&[folder], profile)
            .into_iter()
            .next()
            .unwrap_or(Ok(()))
    }

    /// Resets a single folder to the system default icon.
    ///
    /// Convenience method for resetting a single folder.
    pub fn reset_folder<P: AsRef<Path>>(&self, folder: P) -> Result<()> {
        self.reset_folders(&[folder])
            .into_iter()
            .next()
            .unwrap_or(Ok(()))
    }

    /// Resets the icons for the specified folders to system default with progress reporting.
    ///
    /// This is the async version of [`reset_folders`](Self::reset_folders) that
    /// reports progress through a tokio channel.
    ///
    /// # Arguments
    ///
    /// * `folders` - Collection of folder paths to reset
    /// * `progress` - Channel sender for progress updates
    ///
    /// # Example
    ///
    /// ```ignore
    /// use folco_core::{CustomizationContextBuilder, progress::progress_channel};
    ///
    /// let ctx = CustomizationContextBuilder::new().build()?;
    /// let (tx, mut rx) = progress_channel(32);
    ///
    /// ctx.reset_folders_async(folders, tx).await;
    /// ```
    pub async fn reset_folders_async<P: AsRef<std::path::Path>>(
        &self,
        folders: Vec<P>,
        progress: ProgressSender,
    ) {
        let total = folders.len();

        // Send started event
        let _ = progress.send(Progress::Started { total }).await;

        let mut succeeded = 0usize;
        let mut failed = 0usize;

        // Process each folder
        for (index, folder) in folders.iter().enumerate() {
            let path = folder.as_ref().to_path_buf();

            // Send processing event
            let _ = progress
                .send(Progress::Processing {
                    current: index,
                    path: path.clone(),
                })
                .await;

            // Reset the icon
            match self.folder_provider.reset_icon_for_folder(folder.as_ref()) {
                Ok(()) => {
                    succeeded += 1;
                    let _ = progress
                        .send(Progress::FolderComplete { index, path })
                        .await;
                }
                Err(e) => {
                    failed += 1;
                    let _ = progress
                        .send(Progress::FolderFailed {
                            index,
                            path,
                            error: e.to_string(),
                        })
                        .await;
                }
            }
        }

        // Send completed event
        let _ = progress
            .send(Progress::Completed { succeeded, failed })
            .await;
    }

    /// Clears the icon cache and refreshes from system resources.
    pub fn refresh_cache(&mut self) -> Result<()> {
        let sys_icons = self.cache.refresh()?;
        let renderer_icons = convert_icon_set(&sys_icons);
        let folder_icon_base = FolderIconBase::new(renderer_icons, crate::sys::SURFACE_COLOR);
        self.customizer = FolderIconCustomizer::from_folder(folder_icon_base);
        Ok(())
    }

    /// Customizes the icons for the specified folders with progress reporting.
    ///
    /// This is the async version of [`customize_folders`](Self::customize_folders) that
    /// reports progress through a tokio channel. Use this for GUI progress bars or
    /// CLI progress indicators.
    ///
    /// # Arguments
    ///
    /// * `folders` - Collection of folder paths to customize
    /// * `profile` - The customization profile to apply
    /// * `progress` - Channel sender for progress updates
    ///
    /// # Example
    ///
    /// ```ignore
    /// use folco_core::{CustomizationContextBuilder, progress::progress_channel};
    ///
    /// let mut ctx = CustomizationContextBuilder::new().build()?;
    /// let (tx, mut rx) = progress_channel(32);
    ///
    /// // Handle progress in a separate task
    /// let handle = tokio::spawn(async move {
    ///     while let Some(p) = rx.recv().await {
    ///         println!("{:?}", p);
    ///     }
    /// });
    ///
    /// // Run customization
    /// ctx.customize_folders_async(folders, &profile, tx).await;
    /// handle.await?;
    /// ```
    pub async fn customize_folders_async<P: AsRef<std::path::Path>>(
        &mut self,
        folders: Vec<P>,
        profile: &CustomizationProfile,
        progress: ProgressSender,
    ) {
        let total = folders.len();

        // Send started event
        let _ = progress.send(Progress::Started { total }).await;

        // Apply the profile and render
        let _ = progress.send(Progress::Rendering).await;
        self.apply_profile(profile);
        let rendered = match self.render() {
            Ok(icons) => icons,
            Err(e) => {
                let _ = progress
                    .send(Progress::RenderFailed {
                        error: e.to_string(),
                    })
                    .await;
                let _ = progress
                    .send(Progress::Completed {
                        succeeded: 0,
                        failed: total,
                    })
                    .await;
                return;
            }
        };
        let sys_icons = convert_icon_set_to_sys(&rendered);

        let mut succeeded = 0usize;
        let mut failed = 0usize;

        // Process each folder
        for (index, folder) in folders.iter().enumerate() {
            let path = folder.as_ref().to_path_buf();

            // Send processing event
            let _ = progress
                .send(Progress::Processing {
                    current: index,
                    path: path.clone(),
                })
                .await;

            // Apply the icon
            match self
                .folder_provider
                .set_icon_for_folder(folder.as_ref(), &sys_icons)
            {
                Ok(()) => {
                    succeeded += 1;
                    let _ = progress
                        .send(Progress::FolderComplete { index, path })
                        .await;
                }
                Err(e) => {
                    failed += 1;
                    let _ = progress
                        .send(Progress::FolderFailed {
                            index,
                            path,
                            error: e.to_string(),
                        })
                        .await;
                }
            }
        }

        // Send completed event
        let _ = progress
            .send(Progress::Completed { succeeded, failed })
            .await;
    }

    // ========================================================================
    // Custom Icon Support
    // ========================================================================

    /// Returns the platform size specification for generating custom icon sets.
    ///
    /// Pass the returned specs to [`IconCustomizer::from_image()`] or
    /// [`IconSet::from_image_source()`] to generate platform-compatible sizes.
    pub fn platform_size_spec(&self) -> crate::sys::PlatformSizeSpec {
        crate::sys::PlatformSizeSpec::current_platform()
    }

    /// Creates a [`CustomIconCustomizer`] from a user-provided image.
    ///
    /// Generates the platform-appropriate set of icon sizes automatically.
    /// The returned customizer uses the `Custom` base variant — only
    /// the overlay layer is effective.
    ///
    /// # Errors
    ///
    /// Returns an error if the image source cannot be decoded or rendered.
    pub fn create_custom_icon_customizer(
        &self,
        source: &ImageSource,
    ) -> Result<CustomIconCustomizer> {
        let specs = self.platform_size_spec();
        Ok(CustomIconCustomizer::from_image(source, specs.sizes())?)
    }

    /// Applies custom icons to the specified folders.
    ///
    /// This method:
    /// 1. Renders the custom icon set from the customizer
    /// 2. Converts to system format
    /// 3. Applies to each folder
    ///
    /// # Arguments
    ///
    /// * `folders` - Collection of folder paths to customize
    /// * `customizer` - A configured [`CustomIconCustomizer`] (typically from [`create_custom_icon_customizer`](Self::create_custom_icon_customizer))
    ///
    /// # Returns
    ///
    /// A vector of results, one for each folder.
    pub fn customize_folders_custom<P: AsRef<Path>>(
        &self,
        folders: &[P],
        customizer: &mut CustomIconCustomizer,
    ) -> Vec<Result<()>> {
        // Render the custom icons
        let rendered = match customizer.render_all() {
            Ok(icons) => icons,
            Err(e) => return vec![Err(Error::Render(e))],
        };

        // Convert to system format
        let sys_icons = convert_icon_set_to_sys(&rendered);

        // Apply to each folder
        folders
            .iter()
            .map(|folder| {
                self.folder_provider
                    .set_icon_for_folder(folder.as_ref(), &sys_icons)
                    .map_err(|e| {
                        Error::FolderCustomization(folder.as_ref().to_path_buf(), e.to_string())
                    })
            })
            .collect()
    }

    /// Applies custom icons to the specified folders with progress reporting.
    ///
    /// This is the async version of [`customize_folders_custom`](Self::customize_folders_custom)
    /// that reports progress through a tokio channel.
    ///
    /// # Arguments
    ///
    /// * `folders` - Collection of folder paths to customize
    /// * `customizer` - A configured [`CustomIconCustomizer`] (typically from [`create_custom_icon_customizer`](Self::create_custom_icon_customizer))
    /// * `progress` - Channel sender for progress updates
    ///
    /// # Example
    ///
    /// ```ignore
    /// use folco_core::{CustomizationContextBuilder, ImageSource, progress::progress_channel};
    ///
    /// let ctx = CustomizationContextBuilder::new().build()?;
    /// let source = ImageSource::svg("<svg>...</svg>");
    /// let mut customizer = ctx.create_custom_icon_customizer(&source)?;
    ///
    /// let (tx, mut rx) = progress_channel(32);
    ///
    /// let handle = tokio::spawn(async move {
    ///     while let Some(p) = rx.recv().await {
    ///         println!("{:?}", p);
    ///     }
    /// });
    ///
    /// ctx.customize_folders_custom_async(folders, &mut customizer, tx).await;
    /// handle.await?;
    /// ```
    pub async fn customize_folders_custom_async<P: AsRef<std::path::Path>>(
        &self,
        folders: Vec<P>,
        customizer: &mut CustomIconCustomizer,
        progress: ProgressSender,
    ) {
        let total = folders.len();

        // Send started event
        let _ = progress.send(Progress::Started { total }).await;

        // Render the custom icons
        let _ = progress.send(Progress::Rendering).await;
        let rendered = match customizer.render_all() {
            Ok(icons) => icons,
            Err(e) => {
                let _ = progress
                    .send(Progress::RenderFailed {
                        error: e.to_string(),
                    })
                    .await;
                let _ = progress
                    .send(Progress::Completed {
                        succeeded: 0,
                        failed: total,
                    })
                    .await;
                return;
            }
        };

        // Convert to system format
        let sys_icons = convert_icon_set_to_sys(&rendered);

        let mut succeeded = 0usize;
        let mut failed = 0usize;

        // Process each folder
        for (index, folder) in folders.iter().enumerate() {
            let path = folder.as_ref().to_path_buf();

            // Send processing event
            let _ = progress
                .send(Progress::Processing {
                    current: index,
                    path: path.clone(),
                })
                .await;

            // Apply the icon
            match self
                .folder_provider
                .set_icon_for_folder(folder.as_ref(), &sys_icons)
            {
                Ok(()) => {
                    succeeded += 1;
                    let _ = progress
                        .send(Progress::FolderComplete { index, path })
                        .await;
                }
                Err(e) => {
                    failed += 1;
                    let _ = progress
                        .send(Progress::FolderFailed {
                            index,
                            path,
                            error: e.to_string(),
                        })
                        .await;
                }
            }
        }

        // Send completed event
        let _ = progress
            .send(Progress::Completed { succeeded, failed })
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_has_default_app_info() {
        let builder = CustomizationContextBuilder::new();
        assert_eq!(builder.app_info.qualifier, "com");
        assert_eq!(builder.app_info.organization, "ecoates2");
        assert_eq!(builder.app_info.application, "folco");
    }

    #[test]
    fn test_builder_with_cache_dir() {
        // This test requires a temporary directory but doesn't actually
        // fetch system icons (which would fail in CI)
        let builder = CustomizationContextBuilder::new()
            .with_cache_dir("/tmp/test_folco")
            .with_force_cache_refresh(true);

        assert!(builder.cache_dir.is_some());
        assert!(builder.force_cache_refresh);
    }

    #[test]
    fn test_builder_with_custom_app_info() {
        let builder = CustomizationContextBuilder::new()
            .with_app_info(AppInfo::new("org", "example", "myapp"));

        assert_eq!(builder.app_info.qualifier, "org");
        assert_eq!(builder.app_info.organization, "example");
        assert_eq!(builder.app_info.application, "myapp");
    }

    #[test]
    fn test_app_info_default() {
        let info = AppInfo::default();
        assert_eq!(info.qualifier, "com");
        assert_eq!(info.organization, "ecoates2");
        assert_eq!(info.application, "folco");
    }
}
