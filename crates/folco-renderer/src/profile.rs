//! Serializable customization profile for cross-process/WASM communication.
//!
//! A [`CustomizationProfile`] captures all layer configurations in a format
//! that can be serialized to JSON and sent between frontend (WASM/Tauri) and
//! backend. Layer config structs are directly serializable — no intermediate
//! "settings" types needed.
//!
//! # Example
//!
//! ```
//! use folco_renderer::{CustomizationProfile, SolidColorConfig, DecalConfig, SvgSource};
//!
//! // Build a profile from config structs directly
//! let profile = CustomizationProfile::new()
//!     .with_solid_color(SolidColorConfig::new(33, 150, 243))
//!     .with_decal(DecalConfig::new("<svg>...</svg>", 0.5));
//!
//! // Serialize to JSON for sending to backend
//! let json = profile.to_json().unwrap();
//!
//! // Deserialize in backend
//! let restored = CustomizationProfile::from_json(&json).unwrap();
//! ```

use serde::{Deserialize, Serialize};

use crate::layer::{ColorDotConfig, DecalConfig, ImageOverlayConfig, SolidColorConfig};

// ============================================================================
// CustomizationProfile
// ============================================================================

/// A serializable profile containing all customization configurations.
///
/// This is the primary type for communicating settings between WASM frontend
/// and native backend, and the one profile shape [`Customizer`](crate::Customizer)
/// accepts. Each field stores an optional config struct directly. `Some(config)`
/// means the layer is configured; `None` means it's absent.
///
/// Fields are *intents*, not commitments: `solid_color` and `decal` shift pixels
/// against a surface color, so they apply only to raster folder icons, while
/// `color_dot` and `overlay` draw on top and apply everywhere. Entries the
/// active customizer can't realize are ignored on apply.
///
/// # JSON Format
///
/// ```json
/// {
///   "solidColor": {
///     "targetR": 33,
///     "targetG": 150,
///     "targetB": 243
///   },
///   "colorDot": {
///     "r": 33, "g": 150, "b": 243,
///     "placement": { "position": "bottom-right", "anchorMode": "inset", "scale": 0.4 }
///   },
///   "decal": {
///     "source": { "raw": "<svg>...</svg>" },
///     "scale": 0.5
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CustomizationProfile {
    /// Solid recolor layer config. `None` means not configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solid_color: Option<SolidColorConfig>,

    /// Color dot layer config. `None` means not configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_dot: Option<ColorDotConfig>,

    /// Decal imprint layer config. `None` means not configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decal: Option<DecalConfig>,

    /// Image overlay layer config. `None` means not configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay: Option<ImageOverlayConfig>,
}

impl CustomizationProfile {
    /// Creates an empty profile with no layers configured.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the solid color configuration.
    pub fn with_solid_color(mut self, config: SolidColorConfig) -> Self {
        self.solid_color = Some(config);
        self
    }

    /// Sets the color dot configuration.
    pub fn with_color_dot(mut self, config: ColorDotConfig) -> Self {
        self.color_dot = Some(config);
        self
    }

    /// Sets the decal configuration.
    pub fn with_decal(mut self, config: DecalConfig) -> Self {
        self.decal = Some(config);
        self
    }

    /// Sets the overlay configuration.
    pub fn with_overlay(mut self, config: ImageOverlayConfig) -> Self {
        self.overlay = Some(config);
        self
    }

    /// Whether this profile carries an intent for `layer`.
    pub fn carries(&self, layer: crate::LayerKind) -> bool {
        match layer {
            crate::LayerKind::SolidColor => self.solid_color.is_some(),
            crate::LayerKind::ColorDot => self.color_dot.is_some(),
            crate::LayerKind::Decal => self.decal.is_some(),
            crate::LayerKind::Overlay => self.overlay.is_some(),
        }
    }

    /// Serializes the profile to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serializes the profile to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes a profile from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Returns the JSON Schema for `CustomizationProfile`.
    #[cfg(feature = "jsonschema")]
    pub fn json_schema() -> schemars::schema::RootSchema {
        schemars::schema_for!(CustomizationProfile)
    }

    /// Returns the JSON Schema as a pretty-printed JSON string.
    #[cfg(feature = "jsonschema")]
    pub fn json_schema_string() -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&Self::json_schema())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::{FolderIconBase, IconSet, SurfaceColor};
    use crate::layer::{OverlayAnchorMode, OverlayPosition, SvgSource};
    use crate::placement::Placement;
    use crate::{Customizer, RasterCustomizer};

    fn raster_folder() -> Customizer {
        RasterCustomizer::from_folder(FolderIconBase::new(
            IconSet::new(),
            SurfaceColor::new(255, 217, 112),
        ))
        .into()
    }

    #[test]
    fn profile_serialization_roundtrip() {
        let profile = CustomizationProfile::new()
            .with_solid_color(SolidColorConfig::new(33, 150, 243))
            .with_decal(DecalConfig::new("<svg></svg>", 0.5));

        let json = profile.to_json().unwrap();
        let restored = CustomizationProfile::from_json(&json).unwrap();

        let ct = restored.solid_color.unwrap();
        assert_eq!(ct.target_r, 33);
        assert_eq!(ct.target_g, 150);
        assert_eq!(ct.target_b, 243);

        let decal = restored.decal.unwrap();
        assert_eq!(decal.source, SvgSource::Raw("<svg></svg>".into()));
        assert_eq!(decal.scale, 0.5);

        assert!(restored.overlay.is_none());
    }

    #[test]
    fn profile_json_format() {
        let profile =
            CustomizationProfile::new().with_solid_color(SolidColorConfig::new(76, 175, 80));

        let json = profile.to_json_pretty().unwrap();

        assert!(json.contains("\"solidColor\""));
        assert!(json.contains("\"targetR\""));
        // No "enabled" field
        assert!(!json.contains("\"enabled\""));
    }

    #[test]
    fn color_dot_is_independent_of_solid_color() {
        let profile = CustomizationProfile::new().with_color_dot(ColorDotConfig::new(1, 2, 3));

        let json = profile.to_json().unwrap();
        assert!(json.contains("\"colorDot\""));
        assert!(!json.contains("\"solidColor\""));

        let restored = CustomizationProfile::from_json(&json).unwrap();
        assert_eq!(restored.color_dot.unwrap(), ColorDotConfig::new(1, 2, 3));
        assert!(restored.solid_color.is_none());
    }

    #[test]
    fn overlay_position_serialization() {
        let profile = CustomizationProfile::new().with_overlay(ImageOverlayConfig::from_svg(
            "icon",
            Placement::new(OverlayPosition::TopLeft, OverlayAnchorMode::Inset, 0.25),
        ));

        let json = profile.to_json().unwrap();
        assert!(json.contains("\"top-left\""));

        let restored = CustomizationProfile::from_json(&json).unwrap();
        assert_eq!(
            restored.overlay.unwrap().placement.position,
            OverlayPosition::TopLeft
        );
    }

    #[test]
    fn empty_profile_deserializes() {
        let profile = CustomizationProfile::from_json("{}").unwrap();

        assert!(profile.solid_color.is_none());
        assert!(profile.color_dot.is_none());
        assert!(profile.decal.is_none());
        assert!(profile.overlay.is_none());
    }

    #[test]
    fn profile_apply_to_customizer() {
        let mut customizer = raster_folder();
        customizer.apply_profile(
            CustomizationProfile::new().with_solid_color(SolidColorConfig::new(76, 175, 80)),
        );

        let layers = customizer.layers();
        assert!(layers.solid_color.is_active());
        assert_eq!(layers.solid_color.config().unwrap().target_r, 76);
        // No decal in profile → decal layer stays unconfigured.
        assert!(!layers.decal.is_active());
    }

    #[test]
    fn inactive_layers_export_as_absent() {
        let mut customizer = raster_folder();
        customizer.apply_profile(
            CustomizationProfile::new().with_solid_color(SolidColorConfig::new(76, 175, 80)),
        );
        customizer.apply_profile(CustomizationProfile::new());

        assert!(!customizer.layers().solid_color.is_active());
        assert!(customizer.export_profile().solid_color.is_none());
    }

    /// Export must describe exactly what renders, so feeding it back is a no-op.
    #[test]
    fn export_import_round_trips() {
        let mut source = raster_folder();
        source.apply_profile(
            CustomizationProfile::new()
                .with_solid_color(SolidColorConfig::new(76, 175, 80))
                .with_decal(DecalConfig::new("<svg></svg>", 0.5)),
        );
        source.apply_profile(
            CustomizationProfile::new().with_solid_color(SolidColorConfig::new(76, 175, 80)),
        );

        let exported = source.export_profile();
        let mut restored = raster_folder();
        restored.apply_profile(exported.clone());

        assert_eq!(
            restored.export_profile().to_json().unwrap(),
            exported.to_json().unwrap()
        );
        assert!(restored.layers().solid_color.is_active());
        assert!(!restored.layers().decal.is_active());
    }
}
