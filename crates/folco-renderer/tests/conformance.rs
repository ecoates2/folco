//! Customizer conformance: every customizer must round-trip a profile through
//! `apply_profile` / `export_profile`, keeping exactly the layers its
//! [`IconBaseKind`] can realize and dropping the ones it rejects.
//!
//! This pins the contract a shared customizer trait must preserve.

use folco_renderer::{
    ColorDotConfig, CustomIconCustomizer, CustomProfile, CustomizationProfile, DecalConfig,
    FolderIconBase, FolderIconCustomizer, FolderProfile, IconBaseKind, IconSet,
    ImageOverlayConfig, LayerKind, OverlayAnchorMode, OverlayPosition, SolidColorConfig,
    SurfaceColor, SvgCustomIconCustomizer, SvgFolderIconBase, SvgFolderIconCustomizer,
};

const BASE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" fill="#ffd970"/></svg>"##;

/// A profile carrying every layer, so each customizer has something to reject.
fn full_profile() -> CustomizationProfile {
    CustomizationProfile::new()
        .with_solid_color(SolidColorConfig::new(33, 150, 243))
        .with_decal(DecalConfig::new("<svg xmlns=\"http://www.w3.org/2000/svg\"/>", 0.5))
        .with_color_dot(ColorDotConfig::new(255, 0, 0))
        .with_overlay(ImageOverlayConfig::from_svg(
            "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
            OverlayPosition::TopLeft,
            OverlayAnchorMode::Centered,
            0.3,
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

fn assert_conforms<C>(
    c: &mut C,
    kind: IconBaseKind,
    apply: impl Fn(&mut C, &CustomizationProfile),
    export: impl Fn(&C) -> CustomizationProfile,
) {
    let who = format!("{kind:?}");

    let full = full_profile();
    apply(c, &full);
    assert_profiles_eq(&export(c), &minus_rejected(full, kind), &who);

    // The profile is authoritative: an empty one clears everything.
    apply(c, &CustomizationProfile::new());
    assert_profiles_eq(&export(c), &CustomizationProfile::new(), &who);
}

#[test]
fn raster_folder_conforms() {
    let mut c = FolderIconCustomizer::from_folder(FolderIconBase::new(
        IconSet::new(),
        SurfaceColor::new(255, 217, 112),
    ));
    assert_conforms(
        &mut c,
        FolderIconCustomizer::base_kind(),
        |c, p| c.apply_profile(FolderProfile::from(p)),
        |c| c.export_profile(),
    );
}

#[test]
fn vector_folder_conforms() {
    let mut c = SvgFolderIconCustomizer::from_folder(SvgFolderIconBase::new(BASE_SVG));
    assert_conforms(
        &mut c,
        SvgFolderIconCustomizer::base_kind(),
        |c, p| c.apply_profile(FolderProfile::from(p)),
        |c| c.export_profile(),
    );
}

#[test]
fn raster_custom_image_conforms() {
    let mut c = CustomIconCustomizer::from_icon_set(IconSet::new());
    assert_conforms(
        &mut c,
        CustomIconCustomizer::base_kind(),
        |c, p| c.apply_profile(CustomProfile::from(p)),
        |c| c.export_profile(),
    );
}

#[test]
fn vector_custom_image_conforms() {
    let mut c = SvgCustomIconCustomizer::from_svg(BASE_SVG).unwrap();
    assert_conforms(
        &mut c,
        SvgCustomIconCustomizer::base_kind(),
        |c, p| c.apply_profile(CustomProfile::from(p)),
        |c| c.export_profile(),
    );
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
