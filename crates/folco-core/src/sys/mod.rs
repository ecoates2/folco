//! Platform-specific system icon metadata.
//!
//! This module provides platform-specific information about system folder icons,
//! such as content bounds (the region within an icon image that contains the
//! actual visual content, excluding padding/margins), and platform size
//! specifications for generating multi-resolution icon sets.

use folco_renderer::IconSizeSpec;
use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

// Re-export the platform-specific implementation under a common alias
#[cfg(target_os = "windows")]
pub use windows::get_folder_icon_content_bounds;
#[cfg(target_os = "windows")]
pub use windows::SURFACE_COLOR;

#[cfg(target_os = "macos")]
pub use macos::get_folder_icon_content_bounds;

#[cfg(target_os = "linux")]
pub use linux::get_folder_icon_content_bounds;

// ============================================================================
// PlatformSizeSpec
// ============================================================================

/// Platform-specific icon size specifications.
///
/// Describes the set of sizes an icon should be rasterized to for
/// compatibility with the host operating system. Each platform has
/// its own required set of sizes and scale factors.
///
/// # Example
///
/// ```
/// use folco_core::PlatformSizeSpec;
///
/// let spec = PlatformSizeSpec::current_platform();
/// println!("Platform requires {} icon sizes", spec.sizes().len());
///
/// // Pass to folco-renderer for generating an icon set
/// // let icon_set = IconSet::from_image_source(&source, spec.sizes())?;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSizeSpec {
    pub(super) sizes: Vec<IconSizeSpec>,
}

impl PlatformSizeSpec {
    /// Creates a new size spec from a list of size specifications.
    pub fn new(sizes: Vec<IconSizeSpec>) -> Self {
        Self { sizes }
    }

    /// Returns the size specs as a slice.
    pub fn sizes(&self) -> &[IconSizeSpec] {
        &self.sizes
    }

    /// Returns the size specifications for the current platform.
    ///
    /// - **Windows**: 16, 20, 24, 32, 40, 48, 64, 256 (all @1x)
    /// - **macOS**: 16, 32, 128, 256, 512 (@1x and @2x)
    /// - **Linux**: 16, 22, 24, 32, 48, 64, 128, 256 (all @1x)
    pub fn current_platform() -> Self {
        Self::platform_impl()
    }
}

/// Returns the icon size specifications for the current platform.
///
/// This is a convenience function equivalent to
/// [`PlatformSizeSpec::current_platform()`].
pub fn get_platform_icon_sizes() -> PlatformSizeSpec {
    PlatformSizeSpec::current_platform()
}

#[cfg(test)]
mod platform_size_spec_tests {
    use super::*;

    #[test]
    fn current_platform_returns_sizes() {
        let spec = PlatformSizeSpec::current_platform();
        assert!(!spec.sizes().is_empty());
    }

    #[test]
    fn all_sizes_are_positive() {
        let spec = PlatformSizeSpec::current_platform();
        for size in spec.sizes() {
            assert!(size.width > 0);
            assert!(size.height > 0);
            assert!(size.scale > 0.0);
        }
    }
}
