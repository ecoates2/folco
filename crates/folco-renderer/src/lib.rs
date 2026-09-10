//! folco-renderer: Cross-platform icon customization library
//!
//! This crate provides utilities for loading system icons and applying
//! customizations such as recoloring, color-dot badges, and SVG overlays.
//!
//! # Example
//!
//! ```
//! use folco_renderer::{FolderIconCustomizer, FolderIconBase, IconSet, SolidColorConfig, DecalConfig, SurfaceColor};
//!
//! let surface = SurfaceColor::new(255, 217, 112);
//! let base = FolderIconBase::new(IconSet::new(), surface);
//! let mut customizer = FolderIconCustomizer::from_folder(base);
//!
//! // Configure layers directly through the layers field
//! customizer.layers.solid_color.set_config(Some(SolidColorConfig::new(33, 150, 243)));
//! customizer.layers.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
//!
//! // Toggle layers without losing config
//! customizer.layers.solid_color.set_enabled(false);
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
//!     CustomizationProfile, SolidColorConfig,
//! };
//!
//! let surface = SurfaceColor::new(255, 217, 112);
//! let mut customizer = FolderIconCustomizer::from_folder(FolderIconBase::new(IconSet::new(), surface));
//!
//! // Apply a profile
//! let profile = CustomizationProfile::new()
//!     .with_solid_color(SolidColorConfig::new(33, 150, 243));
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
pub mod medium;
mod profile;
mod svg_folder_customizer;
pub mod svg_layer;

pub use custom_customizer::{CustomIconCustomizer, CustomLayers};
pub use customizer::{IconCustomizer, LayerSet};
pub use error::RenderError;
pub use folder_color::{FolderColor, FolderColorExt, FolderColorMetadata};
pub use folder_customizer::{FolderIconCustomizer, FolderLayers};
pub use icon::{
    FolderIconBase, IconBase, IconImage, IconSet, IconSizeSpec, RectPx, SizePx, SurfaceColor,
    SvgFolderIconBase,
};
pub use layer::{
    CacheKey, ColorDotConfig, DecalConfig, DominantColor, ImageOverlayConfig, ImageSource, Layer,
    LayerConfig, LayerVersions, OverlayAnchorMode, OverlayPosition, RenderContext,
    SolidColorConfig, SvgSource,
};
pub use medium::{Medium, RasterMedium, SvgCanvas, SvgMedium};
pub use profile::CustomizationProfile;
pub use svg_folder_customizer::{SvgFolderIconCustomizer, SvgFolderLayers, SvgLayerSet};
pub use svg_layer::SvgLayer;
