//! Customizer conformance: every base must round-trip a profile through
//! `apply_profile` / `export_profile`, keeping exactly the layers its
//! [`IconBaseKind`] can realize and dropping the ones it rejects.
//!
//! This is the contract the shared [`Customizer`] API exists to preserve.

use folco_renderer::{
    ColorDotConfig, Customizer, CustomizationProfile, DecalConfig, FolderIconBase, IconBaseKind,
    IconSet, ImageOverlayConfig, LayerKind, OverlayAnchorMode, OverlayPosition, Placement,
    RasterCustomizer, SolidColorConfig, SurfaceColor, SvgCustomizer, SvgFolderIconBase,
};

const BASE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" fill="#ffd970"/></svg>"##;

/// A profile carrying every layer, so each base has something to reject.
fn full_profile() -> CustomizationProfile {
    CustomizationProfile::new()
        .with_solid_color(SolidColorConfig::new(33, 150, 243))
        .with_decal(DecalConfig::new("<svg xmlns=\"http://www.w3.org/2000/svg\"/>", 0.5))
        .with_color_dot(ColorDotConfig::new(255, 0, 0))
        .with_overlay(ImageOverlayConfig::from_svg(
            "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
            Placement::new(OverlayPosition::TopLeft, OverlayAnchorMode::Centered, 0.3),
        ))
}

/// `profile` with every layer `kind` rejects cleared.
fn minus_rejected(mut profile: CustomizationProfile, kind: IconBaseKind) -> CustomizationProfile {
    for layer in LayerKind::ALL {
        if kind.rejection(layer).is_none() {
            continue;
        }
        match layer {
            LayerKind::SolidColor => profile.solid_color = None,
            LayerKind::ColorDot => profile.color_dot = None,
            LayerKind::Decal => profile.decal = None,
            LayerKind::Overlay => profile.overlay = None,
        }
    }
    profile
}

/// Configs don't all implement `PartialEq`; the wire format is the contract anyway.
fn assert_profiles_eq(actual: &CustomizationProfile, expected: &CustomizationProfile, who: &str) {
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(expected).unwrap(),
        "{who}: exported profile differs from applied profile minus rejected layers"
    );
}

fn every_base() -> [(IconBaseKind, Customizer); 4] {
    let surface = SurfaceColor::new(255, 217, 112);
    [
        (
            IconBaseKind::RasterFolder,
            RasterCustomizer::from_folder(FolderIconBase::new(IconSet::new(), surface)).into(),
        ),
        (
            IconBaseKind::VectorFolder,
            SvgCustomizer::from_folder(SvgFolderIconBase::new(BASE_SVG)).into(),
        ),
        (
            IconBaseKind::RasterCustomImage,
            RasterCustomizer::from_icon_set(IconSet::new()).into(),
        ),
        (
            IconBaseKind::VectorCustomImage,
            SvgCustomizer::from_svg(BASE_SVG).unwrap().into(),
        ),
    ]
}

#[test]
fn every_base_reports_its_kind() {
    for (kind, c) in every_base() {
        assert_eq!(c.base_kind(), kind);
    }
}

#[test]
fn every_base_keeps_exactly_what_it_can_realize() {
    for (kind, mut c) in every_base() {
        let who = format!("{kind:?}");

        let full = full_profile();
        c.apply_profile(full.clone());
        assert_profiles_eq(&c.export_profile(), &minus_rejected(full, kind), &who);

        // The profile is authoritative: an empty one clears everything.
        c.apply_profile(CustomizationProfile::new());
        assert_profiles_eq(&c.export_profile(), &CustomizationProfile::new(), &who);
    }
}

#[test]
fn every_base_renders_a_preview_after_a_full_profile() {
    for (kind, mut c) in every_base() {
        c.apply_profile(full_profile());
        // Empty raster sets have no sizes to render; only check bases that can.
        if c.base_icons().is_some_and(IconSet::is_empty) {
            continue;
        }
        c.render_preview(32)
            .unwrap_or_else(|e| panic!("{kind:?}: preview failed: {e}"));
    }
}

#[test]
fn full_profile_carries_every_layer() {
    // Guards the fixture itself: if a layer is ever added without updating
    // `full_profile`, the conformance tests above would silently stop covering it.
    let full = full_profile();
    for layer in LayerKind::ALL {
        assert!(full.carries(layer), "fixture is missing {layer}");
    }
}
