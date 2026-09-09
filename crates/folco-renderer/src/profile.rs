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
// CustomizationProfile (folder icons)
// ============================================================================

/// A serializable profile containing all customization configurations.
///
/// This is the primary type for communicating settings between WASM frontend
/// and native backend. Each field stores an optional config struct directly.
/// `Some(config)` means the layer is configured; `None` means it's absent.
///
/// Not every medium supports every layer: `solid_color` recolors pixels and so
/// applies only to raster folder icons, while `color_dot` draws on top and
/// applies everywhere. Unsupported entries are ignored on apply.
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
///   "colorDot": { "r": 33, "g": 150, "b": 243 },
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
// CustomIconProfile (custom images — color dot + overlay)
// ============================================================================

/// A serializable profile for custom icon customization.
///
/// Custom images have no surface color, so only the layers that draw on top
/// (color dot, overlay) are applicable.
///
/// # JSON Format
///
/// ```json
/// {
///   "colorDot": { "r": 33, "g": 150, "b": 243 },
///   "overlay": {
///     "source": { "svg": { "raw": "<svg>...</svg>" } },
///     "position": "bottom-right",
///     "scale": 0.25
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CustomIconProfile {
    /// Color dot layer config. `None` means not configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_dot: Option<ColorDotConfig>,

    /// Image overlay layer config. `None` means not configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay: Option<ImageOverlayConfig>,
}

impl CustomIconProfile {
    /// Creates an empty profile with no layers configured.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the color dot configuration.
    pub fn with_color_dot(mut self, config: ColorDotConfig) -> Self {
        self.color_dot = Some(config);
        self
    }

    /// Sets the overlay configuration.
    pub fn with_overlay(mut self, config: ImageOverlayConfig) -> Self {
        self.overlay = Some(config);
        self
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

    /// Returns the JSON Schema for `CustomIconProfile`.
    #[cfg(feature = "jsonschema")]
    pub fn json_schema() -> schemars::schema::RootSchema {
        schemars::schema_for!(CustomIconProfile)
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
    use crate::layer::{OverlayAnchorMode, OverlayPosition, SvgSource};

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
    fn profile_apply_to_customizer() {
        use crate::FolderIconCustomizer;
        use crate::icon::{FolderIconBase, IconSet, SurfaceColor};

        let profile =
            CustomizationProfile::new().with_solid_color(SolidColorConfig::new(76, 175, 80));
        // No decal in profile → decal layer should be unconfigured

        let mut customizer = FolderIconCustomizer::from_folder(FolderIconBase::new(
            IconSet::new(),
            SurfaceColor::new(255, 217, 112),
        ));
        customizer.apply_profile(&profile);

        assert!(customizer.layers.solid_color.is_active());
        assert_eq!(customizer.layers.solid_color.config().unwrap().target_r, 76);

        assert!(!customizer.layers.decal.has_config());
        assert!(!customizer.layers.decal.is_active());
    }

    #[test]
    fn profile_export_from_customizer() {
        use crate::FolderIconCustomizer;
        use crate::icon::{FolderIconBase, IconSet, SurfaceColor};

        let surface = SurfaceColor::new(255, 217, 112);
        let mut customizer =
            FolderIconCustomizer::from_folder(FolderIconBase::new(IconSet::new(), surface));
        customizer
            .layers
            .solid_color
            .set_config(Some(SolidColorConfig::new(76, 175, 80)));

        let profile = customizer.export_profile();

        let ct = profile.solid_color.unwrap();
        assert_eq!(ct.target_r, 76);
        assert_eq!(ct.target_g, 175);
        assert_eq!(ct.target_b, 80);
        assert!(profile.decal.is_none());
        assert!(profile.overlay.is_none());
    }

    #[test]
    fn overlay_position_serialization() {
        let profile = CustomizationProfile::new().with_overlay(ImageOverlayConfig::from_svg(
            "icon",
            OverlayPosition::TopLeft,
            OverlayAnchorMode::Inset,
            0.25,
        ));

        let json = profile.to_json().unwrap();
        assert!(json.contains("\"top-left\""));

        let restored = CustomizationProfile::from_json(&json).unwrap();
        assert_eq!(restored.overlay.unwrap().position, OverlayPosition::TopLeft);
    }

    #[test]
    fn empty_profile_deserializes() {
        let json = "{}";
        let profile = CustomizationProfile::from_json(json).unwrap();

        assert!(profile.solid_color.is_none());
        assert!(profile.color_dot.is_none());
        assert!(profile.decal.is_none());
        assert!(profile.overlay.is_none());
    }
}
