//! macOS-specific system icon metadata.

use super::PlatformSizeSpec;
use folco_renderer::IconSizeSpec;

impl PlatformSizeSpec {
    pub(super) fn platform_impl() -> Self {
        Self {
            sizes: vec![
                IconSizeSpec::square(16, 1.0),
                IconSizeSpec::square(32, 1.0), // 16@2x
                IconSizeSpec::square(32, 1.0),
                IconSizeSpec::square(64, 2.0), // 32@2x
                IconSizeSpec::square(128, 1.0),
                IconSizeSpec::square(256, 2.0), // 128@2x
                IconSizeSpec::square(256, 1.0),
                IconSizeSpec::square(512, 2.0), // 256@2x
                IconSizeSpec::square(512, 1.0),
                IconSizeSpec::square(1024, 2.0), // 512@2x
            ],
        }
    }
}
