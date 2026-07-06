//! folco-renderer: Cross-platform icon customization library
//!
//! This crate provides utilities for loading system icons and applying
//! customizations such as color targeting and SVG overlays.
//!
//! # Example
//!
//! ```
//! use folco_renderer::{FolderIconCustomizer, FolderIconBase, IconSet, FolderColorTargetConfig, DecalConfig, SurfaceColor};
//!
//! let surface = SurfaceColor::new(255, 217, 112);
//! let base = FolderIconBase::new(IconSet::new(), surface);
//! let mut customizer = FolderIconCustomizer::from_folder(base);
//!
//! // Configure layers directly through the layers field
//! customizer.layers.folder_color_target.set_config(Some(FolderColorTargetConfig::new(33, 150, 243)));
//! customizer.layers.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
//!
//! // Toggle layers without losing config
//! customizer.layers.folder_color_target.set_enabled(false);
//!
//! let output = customizer.render_all();
//! ```
//!
//! # Serializable Profiles
//!
//! For frontend-backend communication, use [`CustomizationProfile`]
//! with the inherent `apply_profile` / `export_profile` methods.
//! For WASM bindings, see the `folco-renderer-wasm` crate.
//!
//! ```
//! use folco_renderer::{
//!     FolderIconCustomizer, FolderIconBase, IconSet, SurfaceColor,
//!     CustomizationProfile, FolderColorTargetConfig,
//! };
//!
//! let surface = SurfaceColor::new(255, 217, 112);
//! let mut customizer = FolderIconCustomizer::from_folder(FolderIconBase::new(IconSet::new(), surface));
//!
//! // Apply a profile
//! let profile = CustomizationProfile::new()
//!     .with_folder_color_target(FolderColorTargetConfig::new(33, 150, 243));
//! customizer.apply_profile(&profile);
//!
//! // Export current settings
//! let exported = customizer.export_profile();
//! let json = exported.to_json().unwrap();
//! ```

mod custom_customizer;
mod customizer;
mod error;
pub mod folder_color;
mod folder_customizer;
mod icon;
pub mod layer;
mod profile;

pub use custom_customizer::{CustomIconCustomizer, OverlayLayers};
pub use customizer::{IconCustomizer, LayerSet};
pub use error::RenderError;
pub use folder_color::{FolderColor, FolderColorMetadata};
pub use folder_customizer::{FolderIconCustomizer, FolderLayers};
pub use icon::{
    FolderIconBase, IconBase, IconImage, IconSet, IconSizeSpec, RectPx, SerializableFolderIconBase,
    SerializableIconImage, SizePx, SurfaceColor,
};
pub use layer::{
    CacheKey, DecalConfig, DominantColor, FolderColorTargetConfig, ImageOverlayConfig, ImageSource,
    Layer, LayerConfig, LayerVersions, OverlayAnchorMode, OverlayPosition, RenderContext,
    SvgSource,
};
pub use profile::{CustomIconProfile, CustomizationProfile};
