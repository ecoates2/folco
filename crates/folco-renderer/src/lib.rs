//! folco-renderer: Cross-platform icon customization library
//!
//! This crate provides utilities for loading system icons and applying
//! customizations such as recoloring, color-dot badges, and SVG overlays.
//!
//! # Example
//!
//! ```
//! use folco_renderer::{Customizer, RasterCustomizer, FolderIconBase, IconSet, SurfaceColor};
//! use folco_renderer::{CustomizationProfile, SolidColorConfig, DecalConfig};
//!
//! let surface = SurfaceColor::new(255, 217, 112);
//! let base = FolderIconBase::new(IconSet::new(), surface);
//! let mut customizer: Customizer = RasterCustomizer::from_folder(base).into();
//!
//! // Apply a profile. Intents this base can't realize are dropped.
//! customizer.apply_profile(
//!     CustomizationProfile::new()
//!         .with_solid_color(SolidColorConfig::new(33, 150, 243))
//!         .with_decal(DecalConfig::new("<svg>...</svg>", 0.5)),
//! );
//!
//! // Export what renders, e.g. to send to a backend.
//! let json = customizer.export_profile().to_json().unwrap();
//!
//! // Render the artifact written to the system, in the base's medium.
//! let output = customizer.render_output();
//! ```
//!
//! [`Customizer`] is the one entry point over both media. Reach for
//! [`RasterCustomizer`] / [`SvgCustomizer`] directly only when you need the
//! concrete pipeline — e.g. for direct access to [`Layers`] in tests.

mod capabilities;
mod customizer;
mod error;
pub mod folder_color;
mod icon;
pub mod layer;
pub mod medium;
mod placement;
mod profile;
mod raster_customizer;
mod svg_customizer;
pub mod svg_layer;

pub use capabilities::{IconBaseKind, IconCapabilities, LayerKind, LayerRejections};
pub use customizer::{Customizer, LayerIntent, Layers, RenderedOutput};
pub use error::RenderError;
pub use folder_color::{FolderColor, FolderColorExt, FolderColorMetadata};
pub use icon::{
    FolderIconBase, IconBase, IconImage, IconSet, IconSizeSpec, RectPx, SizePx, SurfaceColor,
    SvgFolderIconBase,
};
pub use layer::{
    ColorDotConfig, DecalConfig, DominantColor, ImageOverlayConfig, ImageSource, Layer,
    LayerConfig, OverlayAnchorMode, OverlayPosition, RasterPaint, RenderContext,
    SolidColorConfig, Stage, SvgSource,
};
pub use medium::SvgCanvas;
pub use placement::{DefaultPlacements, Placement};
pub use profile::CustomizationProfile;
pub use raster_customizer::RasterCustomizer;
pub use svg_customizer::SvgCustomizer;
pub use svg_layer::{SvgLayer, SvgPaint, SvgStage};
