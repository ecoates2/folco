//! Layer infrastructure for the SVG (vector) rendering medium.
//!
//! SVG layers emit markup fragments into an
//! [`SvgCanvas`](crate::medium::SvgCanvas), which is resolution-independent,
//! so unlike the raster [`Layer`](crate::layer::Layer) there is nothing
//! size-keyed to cache: an SVG layer is just its [`LayerState`].
//!
//! Layer configs are shared with the raster medium (see
//! [`crate::layer`]); only the rendering logic differs, and it lives on the
//! concrete `SvgLayer<Config>` types.

mod color_dot;
mod overlay;

use crate::layer::LayerState;

/// A vector-medium layer: configuration and versioning, no raster cache.
pub type SvgLayer<C> = LayerState<C>;
