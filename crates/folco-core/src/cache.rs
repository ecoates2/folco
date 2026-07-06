//! Icon caching mechanism for system resources.
//!
//! This module provides a caching layer for system folder icons, storing them
//! in the application's data directory to avoid repeatedly extracting them
//! from system resources.

use crate::convert::convert_icon_set;
use crate::error::{Error, Result};

use folco_renderer::IconSet as RendererIconSet;
use icon_sys::IconSet as SysIconSet;
use icon_sys::folder_settings::{DefaultFolderIconProvider, PlatformDefaultFolderIconProvider};

use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for the icon cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// The directory where cached icons are stored.
    pub cache_dir: PathBuf,
    /// Whether to force refresh the cache on next access.
    pub force_refresh: bool,
}

impl CacheConfig {
    /// Creates a new cache configuration with the given cache directory.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            force_refresh: false,
        }
    }

    /// Creates a cache configuration using the standard app data directory.
    ///
    /// Uses `directories::ProjectDirs` to determine the appropriate location
    /// for the current platform.
    ///
    /// # Arguments
    ///
    /// * `qualifier` - The reverse domain qualifier (e.g., "com")
    /// * `organization` - The organization name (e.g., "example")
    /// * `application` - The application name (e.g., "folco")
    pub fn from_app_info(qualifier: &str, organization: &str, application: &str) -> Result<Self> {
        let project_dirs = directories::ProjectDirs::from(qualifier, organization, application)
            .ok_or_else(|| {
                Error::AppDataDir("failed to determine app data directory".to_string())
            })?;

        let cache_dir = project_dirs.data_dir().join("icon_cache");

        Ok(Self::new(cache_dir))
    }

    /// Sets whether to force refresh the cache.
    pub fn with_force_refresh(mut self, force: bool) -> Self {
        self.force_refresh = force;
        self
    }
}

/// Manages caching of system folder icons.
///
/// The cache stores the default system folder icon to avoid repeatedly
/// extracting it from system resources (which can be slow, especially on Windows).
pub struct IconCache {
    config: CacheConfig,
}

impl IconCache {
    /// Creates a new icon cache with the given configuration.
    pub fn new(config: CacheConfig) -> Self {
        Self { config }
    }

    /// Creates a new icon cache using the standard app data directory.
    ///
    /// # Arguments
    ///
    /// * `qualifier` - The reverse domain qualifier (e.g., "com")
    /// * `organization` - The organization name (e.g., "example")
    /// * `application` - The application name (e.g., "folco")
    pub fn from_app_info(qualifier: &str, organization: &str, application: &str) -> Result<Self> {
        let config = CacheConfig::from_app_info(qualifier, organization, application)?;
        Ok(Self::new(config))
    }

    /// Returns the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.config.cache_dir
    }

    /// Ensures the cache directory exists.
    fn ensure_cache_dir(&self) -> Result<()> {
        if !self.config.cache_dir.exists() {
            fs::create_dir_all(&self.config.cache_dir)?;
        }
        Ok(())
    }

    /// Returns the path where a specific icon size would be cached.
    fn icon_path(&self, size: u32, index: usize) -> PathBuf {
        self.config
            .cache_dir
            .join(format!("folder_icon_{}_{}.png", size, index))
    }

    /// Returns the path to the cache manifest file.
    fn manifest_path(&self) -> PathBuf {
        self.config.cache_dir.join("manifest.json")
    }

    /// Checks if a valid cache exists.
    pub fn is_cached(&self) -> bool {
        if self.config.force_refresh {
            return false;
        }
        self.manifest_path().exists()
    }

    /// Gets the default system folder icon, using cache if available.
    ///
    /// Returns the icon set in `icon-sys` format. Use [`Self::get_renderer_icon_set`]
    /// if you need the `folco-renderer` format.
    pub fn get_sys_icon_set(&self) -> Result<SysIconSet> {
        if self.is_cached() {
            self.load_from_cache()
        } else {
            let icon_set = self.fetch_and_cache()?;
            Ok(icon_set)
        }
    }

    /// Gets the default system folder icon in `folco-renderer` format.
    ///
    /// This is the primary method for obtaining icons to use with `FolderIconCustomizer`.
    /// It handles caching automatically and converts to the renderer's format.
    pub fn get_renderer_icon_set(&self) -> Result<RendererIconSet> {
        let sys_set = self.get_sys_icon_set()?;
        Ok(convert_icon_set(&sys_set))
    }

    /// Fetches the system folder icon and caches it.
    fn fetch_and_cache(&self) -> Result<SysIconSet> {
        self.ensure_cache_dir()?;

        // Dump the default folder icon from the system
        let provider = PlatformDefaultFolderIconProvider;
        let icon_set = provider.dump_default_folder_icon()?;

        // Cache each image
        let mut manifest = CacheManifest {
            version: 1,
            icon_count: icon_set.images.len(),
            icons: Vec::new(),
        };

        for (index, image) in icon_set.images.iter().enumerate() {
            let rgba = image.data.to_rgba8();
            let size = rgba.width();
            let path = self.icon_path(size, index);

            rgba.save(&path)?;

            manifest.icons.push(CachedIconInfo {
                size,
                index,
                path: path.to_string_lossy().to_string(),
            });
        }

        // Write manifest
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        fs::write(self.manifest_path(), manifest_json)?;

        Ok(icon_set)
    }

    /// Loads the icon set from cache.
    fn load_from_cache(&self) -> Result<SysIconSet> {
        let manifest_content = fs::read_to_string(self.manifest_path())?;
        let manifest: CacheManifest = serde_json::from_str(&manifest_content)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        let mut images = Vec::with_capacity(manifest.icon_count);

        for info in &manifest.icons {
            let path = PathBuf::from(&info.path);
            if !path.exists() {
                // Cache is invalid, refetch
                return self.fetch_and_cache();
            }

            let img = image::open(&path)?;
            images.push(icon_sys::IconImage { data: img });
        }

        // TODO: Support SVG for linux
        Ok(SysIconSet { images, svg: None })
    }

    /// Clears the cache, forcing a refresh on next access.
    pub fn clear(&self) -> Result<()> {
        if self.config.cache_dir.exists() {
            fs::remove_dir_all(&self.config.cache_dir)?;
        }
        Ok(())
    }

    /// Refreshes the cache by re-fetching from system resources.
    pub fn refresh(&mut self) -> Result<SysIconSet> {
        self.clear()?;
        self.fetch_and_cache()
    }
}

/// Internal manifest format for the cache.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CacheManifest {
    version: u32,
    icon_count: usize,
    icons: Vec<CachedIconInfo>,
}

/// Information about a cached icon.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CachedIconInfo {
    size: u32,
    index: usize,
    path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_config_new() {
        let config = CacheConfig::new("/tmp/test_cache");
        assert_eq!(config.cache_dir, PathBuf::from("/tmp/test_cache"));
        assert!(!config.force_refresh);
    }

    #[test]
    fn test_cache_config_with_force_refresh() {
        let config = CacheConfig::new("/tmp/test_cache").with_force_refresh(true);
        assert!(config.force_refresh);
    }

    #[test]
    fn test_icon_cache_new() {
        let temp_dir = tempdir().unwrap();
        let config = CacheConfig::new(temp_dir.path().join("icons"));
        let cache = IconCache::new(config);

        assert!(!cache.is_cached());
    }

    #[test]
    fn test_ensure_cache_dir() {
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().join("icons").join("nested");
        let config = CacheConfig::new(&cache_path);
        let cache = IconCache::new(config);

        cache.ensure_cache_dir().unwrap();
        assert!(cache_path.exists());
    }
}
