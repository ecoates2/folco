//! Linux-specific system icon metadata.

use super::{PlatformSizeSpec, RasterFolderMetadata, WriteCapabilities};
use folco_renderer::IconSizeSpec;

impl WriteCapabilities {
    // File managers load the generated icon file as-is, SVG or PNG.
    pub(super) fn platform_impl() -> Self {
        Self {
            raster: true,
            vector: true,
        }
    }
}

impl RasterFolderMetadata {
    pub(super) fn platform_impl() -> Option<Self> {
        None
    }
}

impl PlatformSizeSpec {
    pub(super) fn custom_image_impl() -> Self {
        Self {
            sizes: vec![IconSizeSpec::square(512, 1.0)],
        }
    }
}
