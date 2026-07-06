//! Custom icon customizer — overlay-only layers.
//!
//! [`CustomIconCustomizer`] is a type alias for `IconCustomizer<OverlayLayers>`,
//! providing image overlay support on user-supplied base icons.
//!
//! No color target or decal layers are available because custom images
//! have no surface color reference.

use crate::customizer::{IconCustomizer, LayerSet};
use crate::error::RenderError;
use crate::icon::{IconBase, IconSet, IconSizeSpec};
use crate::layer::svg::composite_over;
use crate::layer::{
    CacheKey, DependencyVersion, ImageOverlayConfig, ImageSource, Layer, LayerVersions,
    RenderContext,
};
use crate::profile::CustomIconProfile;

// ============================================================================
// OverlayLayers
// ============================================================================

/// Layer set for custom icon customization — overlay only.
///
/// Custom images have no surface color metadata, so color target and
/// decal layers are not applicable.
#[derive(Default)]
pub struct OverlayLayers {
    /// Image overlay layer.
    pub overlay: Layer<ImageOverlayConfig>,
}

impl LayerSet for OverlayLayers {
    fn execute(&mut self, ctx: &mut RenderContext, key: CacheKey) -> Result<(), RenderError> {
        let versions = LayerVersions {
            folder_color_target: 0,
            decal: 0,
            overlay: self.overlay.version(),
        };

        if let Some(tile) = self.overlay.render_tile(ctx, key, &versions)? {
            composite_over(&mut ctx.image.data, &tile, 0, 0);
        }

        Ok(())
    }

    fn combined_version(&self) -> DependencyVersion {
        DependencyVersion::from_version(self.overlay.version())
    }

    fn invalidate_all(&mut self) {
        self.overlay.invalidate();
    }
}

// ============================================================================
// CustomIconCustomizer
// ============================================================================

/// Custom icon customizer — overlay only, for user-provided images.
///
/// Construct via [`CustomIconCustomizer::from_image()`] or
/// [`CustomIconCustomizer::from_icon_set()`].
///
/// # Example
///
/// ```ignore
/// use folco_renderer::{CustomIconCustomizer, ImageSource, IconSizeSpec, ImageOverlayConfig, OverlayPosition};
///
/// let source = ImageSource::svg("<svg>...</svg>");
/// let specs = vec![IconSizeSpec::square(32, 1.0), IconSizeSpec::square(256, 1.0)];
/// let mut customizer = CustomIconCustomizer::from_image(&source, &specs).unwrap();
///
/// customizer.layers.overlay.set_config(Some(
///     ImageOverlayConfig::from_svg("<svg>badge</svg>", OverlayPosition::BottomRight, OverlayAnchorMode::Inset, 0.25)
/// ));
///
/// let output = customizer.render_all();
/// ```
pub type CustomIconCustomizer = IconCustomizer<OverlayLayers>;

impl CustomIconCustomizer {
    /// Creates a customizer from a user-provided image source.
    ///
    /// Each [`IconSizeSpec`] produces one base image with full-image content
    /// bounds. Only the overlay layer is available.
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be decoded or rendered.
    pub fn from_image(source: &ImageSource, specs: &[IconSizeSpec]) -> Result<Self, RenderError> {
        let icons = IconSet::from_image_source(source, specs)?;
        Ok(IconCustomizer::new(
            IconBase::Custom(icons),
            OverlayLayers::default(),
        ))
    }

    /// Creates a customizer from a pre-built icon set.
    ///
    /// Only the overlay layer is available.
    pub fn from_icon_set(icons: IconSet) -> Self {
        IconCustomizer::new(IconBase::Custom(icons), OverlayLayers::default())
    }

    /// Applies a [`CustomIconProfile`]'s settings to the overlay layer.
    pub fn apply_profile(&mut self, profile: &CustomIconProfile) {
        self.layers.overlay.set_config(profile.overlay.clone());
    }

    /// Exports the current overlay settings as a [`CustomIconProfile`].
    pub fn export_profile(&self) -> CustomIconProfile {
        CustomIconProfile {
            overlay: self.layers.overlay.config().cloned(),
        }
    }
}
