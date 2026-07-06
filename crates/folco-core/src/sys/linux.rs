//! Linux-specific system icon metadata.

use folco_renderer::RectPx;

// Temporary stub to get stuff compiling
pub const SURFACE_COLOR: SurfaceColor = SurfaceColor::new(127, 127, 127);

/// Returns the content bounds for a Linux system folder icon.
///
/// Linux folder icons from icon themes may have specific content regions
/// depending on the theme and icon size.
///
/// # Arguments
///
/// * `width` - The width of the icon image in pixels
/// * `height` - The height of the icon image in pixels
///
/// # Returns
///
/// A `RectPx` describing the region containing the actual icon content.
pub fn get_folder_icon_content_bounds(width: u32, height: u32) -> RectPx {
    // TODO: Determine actual content bounds for Linux folder icons
    unimplemented!(
        "Linux folder icon content bounds not yet implemented for {}x{}",
        width,
        height
    )
}

use super::PlatformSizeSpec;
use folco_renderer::IconSizeSpec;
use folco_renderer::SurfaceColor;

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

#[cfg(test)]
mod tests {
    // Tests will be added once bounds are implemented
}
