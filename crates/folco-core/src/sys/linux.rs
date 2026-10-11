//! Linux-specific system icon metadata.

use super::{PlatformSizeSpec, RasterFolderMetadata};
use folco_renderer::IconSizeSpec;

impl RasterFolderMetadata {
    pub(super) fn platform_impl() -> Option<Self> {
        None
    }
}

impl PlatformSizeSpec {
    pub(super) fn platform_impl() -> Self {
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
