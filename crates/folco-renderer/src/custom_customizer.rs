//! Custom icon customizer — color dot + overlay layers.
//!
//! [`CustomIconCustomizer`] is a type alias for `IconCustomizer<CustomLayers>`,
//! providing color-dot badges and image overlays on user-supplied base icons.
//!
//! The solid-color and decal layers are unavailable because custom images have
//! no surface color reference to recolor against.

use crate::customizer::{IconCustomizer, LayerSet};
use crate::error::RenderError;
use crate::icon::{IconBase, IconSet, IconSizeSpec};
use crate::layer::svg::composite_over;
use crate::layer::{
    CacheKey, ColorDotConfig, DependencyVersion, ImageOverlayConfig, ImageSource, Layer,
    LayerVersions, RenderContext,
};
use crate::profile::CustomIconProfile;

// ============================================================================
// CustomLayers
// ============================================================================

/// Layer set for custom icon customization — color dot and overlay.
///
/// Custom images have no surface color metadata, so the solid-color and decal
/// layers are not applicable.
#[derive(Default)]
pub struct CustomLayers {
    /// Color dot badge layer.
    pub color_dot: Layer<ColorDotConfig>,

    /// Image overlay layer.
    pub overlay: Layer<ImageOverlayConfig>,
}

impl LayerSet for CustomLayers {
    fn execute(&mut self, ctx: &mut RenderContext, key: CacheKey) -> Result<(), RenderError> {
        let versions = LayerVersions {
            solid_color: 0,
            color_dot: self.color_dot.version(),
            decal: 0,
            overlay: self.overlay.version(),
        };

        if let Some(tile) = self.color_dot.render_tile(ctx, key, &versions)? {
            composite_over(&mut ctx.image.data, &tile, 0, 0);
        }

        if let Some(tile) = self.overlay.render_tile(ctx, key, &versions)? {
            composite_over(&mut ctx.image.data, &tile, 0, 0);
        }

        Ok(())
    }

    fn combined_version(&self) -> DependencyVersion {
        DependencyVersion::combine(&[self.color_dot.version(), self.overlay.version()])
    }

    fn invalidate_all(&mut self) {
        self.color_dot.invalidate();
        self.overlay.invalidate();
    }
}

// ============================================================================
// CustomIconCustomizer
// ============================================================================

/// Custom icon customizer — color dot and overlay, for user-provided images.
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
pub type CustomIconCustomizer = IconCustomizer<CustomLayers>;

impl CustomIconCustomizer {
    /// Creates a customizer from a user-provided image source.
    ///
    /// Each [`IconSizeSpec`] produces one base image with full-image content
    /// bounds.
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be decoded or rendered.
    pub fn from_image(source: &ImageSource, specs: &[IconSizeSpec]) -> Result<Self, RenderError> {
        let icons = IconSet::from_image_source(source, specs)?;
        Ok(IconCustomizer::new(
            IconBase::Custom(icons),
            CustomLayers::default(),
        ))
    }

    /// Creates a customizer from a pre-built icon set.
    pub fn from_icon_set(icons: IconSet) -> Self {
        IconCustomizer::new(IconBase::Custom(icons), CustomLayers::default())
    }

    /// Applies a [`CustomIconProfile`]'s settings to the layers.
    pub fn apply_profile(&mut self, profile: &CustomIconProfile) {
        self.layers.color_dot.set_config(profile.color_dot);
        self.layers.overlay.set_config(profile.overlay.clone());
    }

    /// Exports the current settings as a [`CustomIconProfile`].
    pub fn export_profile(&self) -> CustomIconProfile {
        CustomIconProfile {
            color_dot: self.layers.color_dot.config().copied(),
            overlay: self.layers.overlay.config().cloned(),
        }
    }
}
