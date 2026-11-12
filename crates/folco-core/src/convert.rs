//! Conversion from `folco-renderer` output to `icon-sys` format.
//!
//! The opposite direction lives with the base it produces:
//! [`raster_base`](crate::raster_base) and [`svg_base`](crate::svg_base).

use folco_renderer::{IconSet as RendererIconSet, RenderedOutput};
use icon_sys::IconSet as SysIconSet;

/// Trait for types convertible to icon-sys
pub trait SystemFormat {
    fn into_sys(self) -> SysIconSet;
}

impl SystemFormat for RendererIconSet {
    fn into_sys(self) -> SysIconSet {
        let images: Vec<icon_sys::IconImage> = self
            .into_iter()
            .map(|renderer_image| {
                let dynamic = image::DynamicImage::ImageRgba8(renderer_image.data);
                icon_sys::IconImage {
                    data: dynamic,
                    scale: renderer_image.scale,
                }
            })
            .collect();
        SysIconSet { images, svg: None }
    }
}

impl SystemFormat for RenderedOutput {
    fn into_sys(self) -> SysIconSet {
        match self {
            RenderedOutput::Raster(icons) => icons.into_sys(),
            RenderedOutput::Vector(svg) => SysIconSet {
                images: Vec::new(),
                svg: Some(svg),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use folco_renderer::IconImage as RendererIconImage;
    use image::RgbaImage;

    #[test]
    fn test_convert_icon_set_to_sys() {
        // Create a simple renderer icon set
        let rgba = RgbaImage::from_pixel(16, 16, image::Rgba([0, 255, 0, 255]));
        let renderer_img = RendererIconImage::new_full_content(rgba, 1.0);
        let renderer_set = RendererIconSet::from_images(vec![renderer_img]);

        // Convert to sys icon set
        let sys_set = renderer_set.into_sys();

        assert_eq!(sys_set.images.len(), 1);
        let img = &sys_set.images[0];
        assert_eq!(img.data.width(), 16);
        assert_eq!(img.data.height(), 16);
    }

    #[test]
    fn test_renderer_output_to_sys_raster() {
        let rgba = RgbaImage::from_pixel(16, 16, image::Rgba([0, 255, 0, 255]));
        let renderer_img = RendererIconImage::new_full_content(rgba, 1.0);
        let renderer_set = RendererIconSet::from_images(vec![renderer_img]);

        let sys_set = RenderedOutput::Raster(renderer_set).into_sys();
        assert_eq!(sys_set.images.len(), 1);
        assert!(sys_set.svg.is_none());
    }

    #[test]
    fn test_renderer_output_to_sys_vector() {
        let sys_set = RenderedOutput::Vector("<svg/>".to_string()).into_sys();
        assert_eq!(sys_set.images.len(), 0);
        assert_eq!(sys_set.svg.as_deref(), Some("<svg/>"));
    }
}
