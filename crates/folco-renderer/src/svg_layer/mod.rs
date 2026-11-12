//! Layer infrastructure for the SVG (vector) rendering medium.
//!
//! SVG layers emit markup fragments into an
//! [`SvgCanvas`](crate::medium::SvgCanvas), which is resolution-independent,
//! so there is nothing size-keyed to cache: an SVG layer is just a
//! [`Layer`](crate::layer::Layer), same as the raster medium.
//!
//! Layer configs are shared with the raster medium (see
//! [`crate::layer`]); only the rendering differs. A config implements
//! [`SvgPaint`] to produce its fragment, and the blanket [`SvgStage`] impl on
//! `Layer<C>` skips inactive layers.

mod color_dot;
mod overlay;

use crate::error::RenderError;
use crate::layer::{Layer, LayerConfig};

/// A vector-medium layer: configuration and versioning, no raster cache.
pub type SvgLayer<C> = Layer<C>;

/// How a config renders itself as SVG markup.
pub trait SvgPaint {
    /// The fragment to inject before the base document's closing tag.
    fn fragment(&self) -> Result<String, RenderError>;
}

/// One step of the vector pipeline, as the customizer sees it.
pub trait SvgStage {
    /// Changes whenever this stage's output could change.
    fn version(&self) -> u64;

    /// The fragment to append, or `None` if the layer is inactive.
    fn fragment(&self) -> Result<Option<String>, RenderError>;
}

impl<C: LayerConfig + SvgPaint> SvgStage for Layer<C> {
    fn version(&self) -> u64 {
        Layer::version(self)
    }

    fn fragment(&self) -> Result<Option<String>, RenderError> {
        self.config().map(SvgPaint::fragment).transpose()
    }
}
