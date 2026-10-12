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

    pub(super) fn system_folder_icon_impl() -> Self {
        Self {
            sizes: vec![
                IconSizeSpec::square(16, 1.0),
                IconSizeSpec::square(22, 1.0),
                IconSizeSpec::square(24, 1.0),
                IconSizeSpec::square(32, 1.0),
                IconSizeSpec::square(48, 1.0),
                IconSizeSpec::square(64, 1.0),
                IconSizeSpec::square(128, 1.0),
                IconSizeSpec::square(256, 1.0),
            ],
        }
    }
}
