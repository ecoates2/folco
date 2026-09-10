//! Which layers a resolved icon base can actually realize.
//!
//! Capabilities are a property of the **resolved base**, never of the host
//! platform. On Linux the folder icon's medium depends on the installed icon
//! theme — Adwaita ships SVG, other themes ship PNGs — so this is only knowable
//! once the icon is in hand.
//!
//! [`IconBaseKind`] names what a customization resolved to operate on, and is
//! the single source of truth for the rules that follow from it. Everything
//! that needs to know "can this base take a decal?" derives it from here rather
//! than re-deciding: the customizers when dropping profile intents, `folco-core`
//! when pre-flighting a profile, and the wasm bindings when telling a UI which
//! controls to disable.

use serde::{Deserialize, Serialize};

use crate::profile::CustomizationProfile;

// ============================================================================
// LayerKind
// ============================================================================

/// A layer a [`CustomizationProfile`] can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum LayerKind {
    /// Recolors the whole icon by HSL-shifting against the surface color.
    SolidColor,
    /// A colored badge in the bottom-right corner.
    ColorDot,
    /// A tinted glyph imprinted into the icon's face.
    Decal,
    /// An image composited on top of the icon.
    Overlay,
}

impl LayerKind {
    /// Every layer, in pipeline order.
    pub const ALL: [LayerKind; 4] = [
        LayerKind::SolidColor,
        LayerKind::ColorDot,
        LayerKind::Decal,
        LayerKind::Overlay,
    ];
}

impl std::fmt::Display for LayerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SolidColor => "solid color",
            Self::ColorDot => "color dot",
            Self::Decal => "decal",
            Self::Overlay => "overlay",
        })
    }
}

// ============================================================================
// IconBaseKind
// ============================================================================

/// What a customization resolved to operate on.
///
/// Two axes collapse into this one value: where the base came from (the system
/// folder icon, or the user) and what medium it arrived in. Both are runtime
/// facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum IconBaseKind {
    /// The system folder icon, delivered as a PNG set.
    RasterFolder,
    /// The system folder icon, delivered as scalable markup.
    VectorFolder,
    /// An image the user supplied.
    CustomImage,
}

impl IconBaseKind {
    /// The layers this base can realize.
    pub fn capabilities(self) -> IconCapabilities {
        // Solid color and decal both shift pixels against a known surface
        // color, so they need a raster base *and* a surface color to shift
        // against. Only the system's raster folder icon has both.
        let surface_relative = matches!(self, Self::RasterFolder);
        IconCapabilities {
            solid_color: surface_relative,
            color_dot: true,
            decal: surface_relative,
            overlay: true,
        }
    }

    /// Why this base can't realize `layer`, or `None` when it can.
    ///
    /// Phrased for end users; callers surface it verbatim.
    pub fn rejection(self, layer: LayerKind) -> Option<&'static str> {
        if self.capabilities().supports(layer) {
            return None;
        }
        Some(match self {
            Self::VectorFolder => match layer {
                LayerKind::SolidColor => {
                    "this system's folder icon is a scalable SVG, which has no pixels to recolor"
                }
                _ => {
                    "this system's folder icon is a scalable SVG, and decals are imprinted per-pixel"
                }
            },
            Self::CustomImage => match layer {
                LayerKind::SolidColor => {
                    "a custom image has no known surface color to recolor away from"
                }
                _ => {
                    "decals are tinted against a folder's surface color, which a custom image has none of"
                }
            },
            // Realizes everything, so `supports` already returned above.
            Self::RasterFolder => unreachable!(),
        })
    }

    /// Why each layer this base can't realize is unavailable.
    ///
    /// A `None` entry means the layer works. Built for UIs that need to explain
    /// a disabled control rather than silently hide it.
    pub fn rejections(self) -> LayerRejections {
        LayerRejections {
            solid_color: self.rejection(LayerKind::SolidColor).map(str::to_owned),
            color_dot: self.rejection(LayerKind::ColorDot).map(str::to_owned),
            decal: self.rejection(LayerKind::Decal).map(str::to_owned),
            overlay: self.rejection(LayerKind::Overlay).map(str::to_owned),
        }
    }
}

// ============================================================================
// LayerRejections
// ============================================================================

/// Per-layer explanations for why the resolved base can't realize a layer.
///
/// Mirrors [`IconCapabilities`] field for field: where the capability is
/// `false`, the matching entry here says why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct LayerRejections {
    pub solid_color: Option<String>,
    pub color_dot: Option<String>,
    pub decal: Option<String>,
    pub overlay: Option<String>,
}

// ============================================================================
// IconCapabilities
// ============================================================================

/// Which layers the resolved base can realize.
///
/// Obtain one from [`IconBaseKind::capabilities`] rather than building it by
/// hand — the rules belong to the base kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(into_wasm_abi, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct IconCapabilities {
    pub solid_color: bool,
    pub color_dot: bool,
    pub decal: bool,
    pub overlay: bool,
}

impl IconCapabilities {
    /// Whether `layer` can be realized.
    pub fn supports(&self, layer: LayerKind) -> bool {
        match layer {
            LayerKind::SolidColor => self.solid_color,
            LayerKind::ColorDot => self.color_dot,
            LayerKind::Decal => self.decal,
            LayerKind::Overlay => self.overlay,
        }
    }

    /// The layers `profile` carries that this base can't realize, in pipeline order.
    ///
    /// Empty means the profile applies in full.
    pub fn unsupported_in(&self, profile: &CustomizationProfile) -> Vec<LayerKind> {
        LayerKind::ALL
            .into_iter()
            .filter(|&layer| profile.carries(layer) && !self.supports(layer))
            .collect()
    }

    /// A copy of `profile` with unrealizable intents removed.
    pub fn filter(&self, profile: &CustomizationProfile) -> CustomizationProfile {
        CustomizationProfile {
            solid_color: self
                .solid_color
                .then(|| profile.solid_color.clone())
                .flatten(),
            color_dot: self.color_dot.then_some(profile.color_dot).flatten(),
            decal: self.decal.then(|| profile.decal.clone()).flatten(),
            overlay: self.overlay.then(|| profile.overlay.clone()).flatten(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{
        ColorDotConfig, DecalConfig, ImageOverlayConfig, OverlayAnchorMode, OverlayPosition,
        SolidColorConfig,
    };

    fn full_profile() -> CustomizationProfile {
        CustomizationProfile::new()
            .with_solid_color(SolidColorConfig::new(1, 2, 3))
            .with_color_dot(ColorDotConfig::new(4, 5, 6))
            .with_decal(DecalConfig::new("<svg></svg>", 0.5))
            .with_overlay(ImageOverlayConfig::from_svg(
                "<svg></svg>",
                OverlayPosition::TopLeft,
                OverlayAnchorMode::Inset,
                0.25,
            ))
    }

    #[test]
    fn raster_folder_realizes_everything() {
        let caps = IconBaseKind::RasterFolder.capabilities();
        assert!(LayerKind::ALL.iter().all(|&l| caps.supports(l)));
        assert!(caps.unsupported_in(&full_profile()).is_empty());
    }

    #[test]
    fn surface_relative_layers_need_a_raster_folder() {
        for kind in [IconBaseKind::VectorFolder, IconBaseKind::CustomImage] {
            assert_eq!(
                kind.capabilities().unsupported_in(&full_profile()),
                vec![LayerKind::SolidColor, LayerKind::Decal],
                "{kind:?}"
            );
        }
    }

    #[test]
    fn unsupported_only_reports_layers_the_profile_carries() {
        let profile = CustomizationProfile::new().with_decal(DecalConfig::new("<svg></svg>", 0.5));
        assert_eq!(
            IconBaseKind::VectorFolder
                .capabilities()
                .unsupported_in(&profile),
            vec![LayerKind::Decal]
        );
    }

    #[test]
    fn filter_drops_exactly_the_unsupported_layers() {
        let filtered = IconBaseKind::CustomImage
            .capabilities()
            .filter(&full_profile());

        assert!(filtered.solid_color.is_none());
        assert!(filtered.decal.is_none());
        assert!(filtered.color_dot.is_some());
        assert!(filtered.overlay.is_some());
    }

    #[test]
    fn rejection_is_present_exactly_when_unsupported() {
        for kind in [
            IconBaseKind::RasterFolder,
            IconBaseKind::VectorFolder,
            IconBaseKind::CustomImage,
        ] {
            let caps = kind.capabilities();
            for layer in LayerKind::ALL {
                assert_eq!(
                    kind.rejection(layer).is_none(),
                    caps.supports(layer),
                    "{kind:?} / {layer}"
                );
            }
        }
    }

    /// The invariant that keeps every consumer of capabilities honest: what a
    /// customizer actually keeps must equal what its capabilities advertise.
    #[test]
    fn capabilities_match_what_customizers_keep() {
        use crate::icon::{FolderIconBase, IconSet, SurfaceColor, SvgFolderIconBase};
        use crate::{CustomIconCustomizer, FolderIconCustomizer, SvgFolderIconCustomizer};

        fn assert_keeps_exactly(caps: IconCapabilities, exported: &CustomizationProfile) {
            for layer in LayerKind::ALL {
                assert_eq!(exported.carries(layer), caps.supports(layer), "{layer}");
            }
        }

        let profile = full_profile();
        let surface = SurfaceColor::new(255, 217, 112);

        let mut folder =
            FolderIconCustomizer::from_folder(FolderIconBase::new(IconSet::new(), surface));
        folder.apply_profile(&profile);
        assert_keeps_exactly(
            FolderIconCustomizer::capabilities(),
            &folder.export_profile(),
        );

        let mut svg = SvgFolderIconCustomizer::from_folder(SvgFolderIconBase::new(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"></svg>"#,
            surface,
        ));
        svg.apply_profile(&profile);
        assert_keeps_exactly(
            SvgFolderIconCustomizer::capabilities(),
            &svg.export_profile(),
        );

        let mut custom = CustomIconCustomizer::from_icon_set(IconSet::new());
        custom.apply_profile(&profile);
        assert_keeps_exactly(
            CustomIconCustomizer::capabilities(),
            &custom.export_profile(),
        );
    }
}
