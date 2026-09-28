use crate::{IconImage, RenderError};

pub trait Render {
    type Output;

    /// Render all sizes / the full output.
    fn render_full_output(&mut self) -> Result<Self::Output, RenderError>;

    /// Render a single size for preview/display.
    fn render_raster_preview(&mut self, size: u32) -> Result<IconImage, RenderError>;
}
