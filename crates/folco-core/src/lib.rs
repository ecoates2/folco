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
//! use folco_core::{CustomizationContext, CustomizationContextBuilder};
//! use std::path::PathBuf;
//!
//! // Initialize with cached system icons
//! let ctx = CustomizationContextBuilder::new()
//!     .with_app_info("com", "example", "folco")
//!     .build()?;
//!
//! // Customize folders with a profile
//! let folders = vec![PathBuf::from("/path/to/folder")];
//! ctx.customize_folders(&folders, &profile)?;
//!
//! // Reset folders to default
//! ctx.reset_folders(&folders)?;
//! ```

mod cache;
mod context;
mod convert;
mod error;
pub mod folder_color;
pub mod progress;
mod sys;

pub use cache::{CacheConfig, IconCache};
pub use context::{AppInfo, CustomizationContext, CustomizationContextBuilder};
pub use convert::convert_icon_set;
pub use error::{Error, Result};
pub use sys::{PlatformSizeSpec, get_platform_icon_sizes};

// Re-export key types from folco-renderer for convenience
// This allows consumers to use profiles without importing the renderer crate directly
pub use folco_renderer::{
    CustomIconCustomizer, CustomIconProfile, CustomizationProfile, DecalConfig, FolderColor,
    FolderColorExt, FolderColorMetadata, FolderColorTargetConfig, FolderIconBase,
    FolderIconCustomizer, FolderLayers, IconBase, IconCustomizer, IconSizeSpec, ImageOverlayConfig,
    ImageSource, LayerSet, OverlayAnchorMode, OverlayLayers, OverlayPosition, RectPx, SurfaceColor,
    SvgSource,
};
