//! folco-core: Core library for folder icon customization
//!
//! This crate provides the glue between `icon-sys` (system icon operations)
//! and `folco-renderer` (icon customization rendering). It's designed to be
//! consumed by both `folco-gui` (Tauri app) and `folco-cli`.
//!
//! # Features
//!
//! - **CustomizationContext**: Main entry point for all icon customization operations
//! - **Folder customization**: Apply custom icons to directories
//! - **Reset to default**: Restore system default folder icons
//! - **Icon caching**: Cache system resources in app data directory
//! - **Type conversion**: Convert between `icon-sys` and `folco-renderer` icon types
//!
//! # Example
//!
//! ```ignore
//! use folco_core::{CustomizationContextBuilder, CustomizationProfile, SolidColorConfig};
//! use folco_core::progress::progress_channel;
//! use std::path::PathBuf;
//!
//! // Initialize with cached system icons
//! let mut ctx = CustomizationContextBuilder::new().build()?;
//!
//! // Customize folders with a profile; per-folder outcomes arrive on `rx`
//! let folders = vec![PathBuf::from("/path/to/folder")];
//! let profile = CustomizationProfile::new()
//!     .with_solid_color(SolidColorConfig::new(33, 150, 243));
//! let (tx, _rx) = progress_channel(32);
//! ctx.customize_folders_async(folders.clone(), profile, tx).await;
//!
//! // Reset folders to default
//! let (tx, _rx) = progress_channel(32);
//! ctx.reset_folders_async(folders, tx).await;
//! ```

mod cache;
mod context;
mod convert;
mod error;
pub mod experimental;
pub mod folder_color;
mod image_file;
pub mod progress;
mod raster_base;
mod svg_base;
mod sys;

pub use cache::{CacheConfig, IconCache};
pub use context::{AppInfo, CustomizationContext, CustomizationContextBuilder, UnsupportedLayer};
pub use error::{Error, Result};
pub use image_file::{ImageFile, ImageFormat};
pub use raster_base::convert_icon_set;
pub use sys::{OutputMedium, PlatformSizeSpec, WriteCapabilities};

// Re-export key types from folco-renderer for convenience
// This allows consumers to use profiles without importing the renderer crate directly
pub use folco_renderer::{
    ColorDotConfig, CustomIconCustomizer, CustomLayers, CustomProfile, CustomizationProfile,
    DecalConfig, DefaultPlacements, FolderColor, FolderColorExt, FolderColorMetadata,
    FolderIconBase, FolderIconCustomizer, FolderLayers, FolderProfile, IconBase, IconBaseKind,
    IconCapabilities, IconCustomizer, IconSizeSpec, ImageOverlayConfig, ImageSource, LayerKind,
    LayerSet, OverlayAnchorMode, OverlayPosition, Placement, RectPx, SolidColorConfig,
    SurfaceColor, SvgSource,
};
