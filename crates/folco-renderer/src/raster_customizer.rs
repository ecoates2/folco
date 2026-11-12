//! Raster customizer: the pixel pipeline over a PNG icon set.
//!
//! [`RasterCustomizer`] runs the four raster layers as stages and caches the
//! [`RenderContext`] after each one, per size, so a change to layer *k*
//! re-runs only stages *k* and later. The base can be a system folder icon
//! (with a surface color, so every layer applies) or a user-supplied image
//! (draw-on-top layers only).

use std::collections::HashMap;
use std::sync::Arc;

use crate::capabilities::IconBaseKind;
use crate::customizer::Layers;
use crate::error::RenderError;
use crate::icon::{FolderIconBase, IconBase, IconImage, IconSet, IconSizeSpec, SurfaceColor};
use crate::layer::{CacheKey, ImageSource, RenderContext, Stage};

// ============================================================================
// StageCache
// ============================================================================

/// Context snapshots after each stage, per base size.
///
/// `snapshots[i]` is the context after stage `i` ran at `versions[i]`. A
/// render reuses the longest prefix whose versions still match and rebuilds
/// from there, so editing the last layer costs exactly one stage.
///
/// Snapshots are `Arc`-shared: an inactive stage's snapshot *is* the previous
/// one, and a rebuild copies pixels only when a stage actually paints.
#[derive(Default)]
struct StageCache {
    by_size: HashMap<CacheKey, Vec<Snapshot>>,
}

struct Snapshot {
    version: u64,
    ctx: Arc<RenderContext>,
}

impl StageCache {
    /// Renders `base` through `stages`, reusing cached prefixes where valid.
    ///
    /// Returns a reference into the cache (or `base` itself for an empty
    /// pipeline), so a cache hit copies nothing.
    fn render<'a>(
        &'a mut self,
        base: &'a IconImage,
        surface_color: Option<SurfaceColor>,
        stages: &[&dyn Stage],
    ) -> Result<&'a IconImage, RenderError> {
        let snapshots = self.by_size.entry(CacheKey::from_icon(base)).or_default();

        let valid = snapshots
            .iter()
            .zip(stages)
            .take_while(|(snap, stage)| snap.version == stage.version())
            .count();
        snapshots.truncate(valid);

        if valid < stages.len() {
            let mut ctx = match snapshots.last() {
                Some(snap) => Arc::clone(&snap.ctx),
                None => Arc::new(RenderContext {
                    image: base.clone(),
                    surface_color,
                    dominant_color: None,
                }),
            };

            for stage in &stages[valid..] {
                if stage.is_active() {
                    // Copies the shared snapshot only on the first write after reuse.
                    stage.run(Arc::make_mut(&mut ctx))?;
                }
                snapshots.push(Snapshot {
                    version: stage.version(),
                    ctx: Arc::clone(&ctx),
                });
            }
        }

        Ok(match snapshots.last() {
            Some(snap) => &snap.ctx.image,
            None => base,
        })
    }
}

// ============================================================================
// RasterCustomizer
// ============================================================================

/// Pixel pipeline over a raster icon set.
///
/// Configure layers through the [`layers`](Self::layers) field, or through
/// [`Customizer`](crate::Customizer) for the capability-checked API shared
/// with the vector pipeline.
///
/// # Example
///
/// ```
/// use folco_renderer::{RasterCustomizer, FolderIconBase, IconSet, SolidColorConfig, DecalConfig, SurfaceColor};
///
/// let surface = SurfaceColor::new(255, 217, 112);
/// let base = FolderIconBase::new(IconSet::new(), surface);
/// let mut customizer = RasterCustomizer::from_folder(base);
///
/// customizer.layers.solid_color.set_config(Some(SolidColorConfig::new(33, 150, 243)));
/// customizer.layers.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
///
/// let output = customizer.render_output();
/// ```
pub struct RasterCustomizer {
    base: IconBase,

    /// The layers. Stages run in field order: solid color, decal, color dot, overlay.
    pub layers: Layers,

    cache: StageCache,
}

impl RasterCustomizer {
    fn new(base: IconBase) -> Self {
        Self {
            base,
            layers: Layers::default(),
            cache: StageCache::default(),
        }
    }

    /// Creates a customizer for system folder icons. All layers apply.
    pub fn from_folder(base: FolderIconBase) -> Self {
        Self::new(IconBase::Folder(base))
    }

    /// Creates a customizer from a user-provided image source.
    ///
    /// Each [`IconSizeSpec`] produces one square base image; non-square sources
    /// are fit and centered, with content bounds covering the placed image.
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be decoded or rendered.
    pub fn from_image(source: &ImageSource, specs: &[IconSizeSpec]) -> Result<Self, RenderError> {
        let icons = IconSet::from_image_source(source, specs)?;
        Ok(Self::from_icon_set(icons))
    }

    /// Creates a customizer over a pre-built user-provided icon set.
    pub fn from_icon_set(icons: IconSet) -> Self {
        Self::new(IconBase::Custom(icons))
    }

    // ---- Accessors ----------------------------------------------------------

    /// What this customizer operates on.
    pub fn base_kind(&self) -> IconBaseKind {
        match self.base {
            IconBase::Folder(_) => IconBaseKind::RasterFolder,
            IconBase::Custom(_) => IconBaseKind::RasterCustomImage,
        }
    }

    /// Returns a reference to the icon base.
    pub fn base(&self) -> &IconBase {
        &self.base
    }

    /// Returns a reference to the base icon set.
    pub fn base_icons(&self) -> &IconSet {
        self.base.icons()
    }

    /// Returns the surface color, if the base is a system folder icon.
    pub fn surface_color(&self) -> Option<&SurfaceColor> {
        self.base.surface_color()
    }

    // ---- Rendering ----------------------------------------------------------

    /// The raster pipeline, in order. The decal must see solid color's output.
    fn stages(layers: &Layers) -> [&dyn Stage; 4] {
        [
            &layers.solid_color,
            &layers.decal,
            &layers.color_dot,
            &layers.overlay,
        ]
    }

    /// Renders a single icon at the specified logical size.
    ///
    /// Returns the closest matching size from the base icon set,
    /// with all applicable customizations applied. The image is borrowed from
    /// the cache; clone it if it must outlive the next edit.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::NoBaseIcon`] if no base icon matches the size,
    /// or a render error if a layer fails.
    pub fn render_preview(&mut self, logical_size: u32) -> Result<&IconImage, RenderError> {
        let icon = self
            .base
            .icons()
            .find_by_logical_size(logical_size)
            .ok_or(RenderError::NoBaseIcon { logical_size })?;
        self.cache.render(
            icon,
            self.base.surface_color().copied(),
            &Self::stages(&self.layers),
        )
    }

    /// Renders every size in the base icon set with customizations applied.
    ///
    /// # Errors
    ///
    /// Returns a render error if any layer fails.
    pub fn render_output(&mut self) -> Result<IconSet, RenderError> {
        let surface = self.base.surface_color().copied();
        let stages = Self::stages(&self.layers);
        let cache = &mut self.cache;
        let rendered = self
            .base
            .icons()
            .iter()
            // The cache keeps its copy; the output set needs its own.
            .map(|icon| cache.render(icon, surface, &stages).cloned())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(IconSet::from_images(rendered))
    }

    /// Drops every cached stage snapshot.
    pub fn clear_cache(&mut self) {
        self.cache.by_size.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::RectPx;
    use crate::layer::decal::darken_color;
    use crate::layer::{
        ColorDotConfig, DecalConfig, ImageOverlayConfig, Layer, OverlayAnchorMode,
        OverlayPosition, SolidColorConfig,
    };
    use crate::placement::Placement;
    use image::RgbaImage;
    use std::cell::Cell;

    // ---- StageCache ---------------------------------------------------------

    /// A stage that records how often it ran and paints its id into pixel (0, 0).
    struct Counting {
        id: u8,
        active: bool,
        version: Cell<u64>,
        runs: Cell<usize>,
    }

    impl Counting {
        fn new(id: u8) -> Self {
            Self {
                id,
                active: true,
                version: Cell::new(0),
                runs: Cell::new(0),
            }
        }

        fn inactive(id: u8) -> Self {
            Self {
                active: false,
                ..Self::new(id)
            }
        }

        fn bump(&self) {
            self.version.set(self.version.get() + 1);
        }
    }

    impl Stage for Counting {
        fn version(&self) -> u64 {
            self.version.get()
        }

        fn is_active(&self) -> bool {
            self.active
        }

        fn run(&self, ctx: &mut RenderContext) -> Result<(), RenderError> {
            self.runs.set(self.runs.get() + 1);
            let px = ctx.image.data.get_pixel_mut(0, 0);
            px.0[self.id as usize] = px.0[self.id as usize].wrapping_add(1);
            Ok(())
        }
    }

    fn blank(size: u32) -> IconImage {
        IconImage::new(
            RgbaImage::new(size, size),
            1.0,
            RectPx::from_size(size, size),
        )
    }

    fn runs(stages: &[Counting]) -> Vec<usize> {
        stages.iter().map(|s| s.runs.get()).collect()
    }

    #[test]
    fn unchanged_pipeline_runs_nothing_twice() {
        let stages = [Counting::new(0), Counting::new(1), Counting::new(2)];
        let refs: Vec<&dyn Stage> = stages.iter().map(|s| s as &dyn Stage).collect();
        let mut cache = StageCache::default();
        let base = blank(8);

        let first = cache.render(&base, None, &refs).unwrap().clone();
        let second = cache.render(&base, None, &refs).unwrap();

        assert_eq!(runs(&stages), [1, 1, 1]);
        assert_eq!(first.data.as_raw(), second.data.as_raw());
        // Each stage painted exactly once into the cached image.
        assert_eq!(first.data.get_pixel(0, 0).0, [1, 1, 1, 0]);
    }

    #[test]
    fn inactive_stages_are_skipped_and_share_the_previous_snapshot() {
        let stages = [Counting::new(0), Counting::inactive(1), Counting::new(2)];
        let refs: Vec<&dyn Stage> = stages.iter().map(|s| s as &dyn Stage).collect();
        let mut cache = StageCache::default();
        let base = blank(8);

        cache.render(&base, None, &refs).unwrap();
        assert_eq!(runs(&stages), [1, 0, 1]);

        let snaps = &cache.by_size[&CacheKey::from_icon(&base)];
        assert!(
            Arc::ptr_eq(&snaps[0].ctx, &snaps[1].ctx),
            "inactive stage must not copy the context"
        );
        assert!(
            !Arc::ptr_eq(&snaps[1].ctx, &snaps[2].ctx),
            "active stage must get its own copy"
        );
    }

    #[test]
    fn changing_a_stage_reruns_only_it_and_later_stages() {
        let stages = [Counting::new(0), Counting::new(1), Counting::new(2)];
        let refs: Vec<&dyn Stage> = stages.iter().map(|s| s as &dyn Stage).collect();
        let mut cache = StageCache::default();
        let base = blank(8);

        cache.render(&base, None, &refs).unwrap();
        stages[1].bump();
        let image = cache.render(&base, None, &refs).unwrap();

        assert_eq!(runs(&stages), [1, 2, 2]);
        // Stage 0's output was reused, not re-painted on top of itself.
        assert_eq!(image.data.get_pixel(0, 0).0, [1, 1, 1, 0]);
    }

    #[test]
    fn changing_the_last_stage_costs_one_stage() {
        let stages = [Counting::new(0), Counting::new(1), Counting::new(2)];
        let refs: Vec<&dyn Stage> = stages.iter().map(|s| s as &dyn Stage).collect();
        let mut cache = StageCache::default();

        cache.render(&blank(8), None, &refs).unwrap();
        stages[2].bump();
        cache.render(&blank(8), None, &refs).unwrap();

        assert_eq!(runs(&stages), [1, 1, 2]);
    }

    #[test]
    fn sizes_are_cached_independently() {
        let stages = [Counting::new(0)];
        let refs: Vec<&dyn Stage> = stages.iter().map(|s| s as &dyn Stage).collect();
        let mut cache = StageCache::default();

        cache.render(&blank(8), None, &refs).unwrap();
        cache.render(&blank(16), None, &refs).unwrap();
        cache.render(&blank(8), None, &refs).unwrap();

        assert_eq!(runs(&stages), [2]);
    }

    #[test]
    fn empty_pipeline_returns_the_base() {
        let mut cache = StageCache::default();
        let base = blank(8);
        let image = cache.render(&base, None, &[]).unwrap();
        assert!(std::ptr::eq(image, &base), "nothing to do, nothing to copy");
    }

    #[test]
    fn surface_color_reaches_the_first_stage() {
        struct Probe(Cell<Option<SurfaceColor>>);
        impl Stage for Probe {
            fn version(&self) -> u64 {
                0
            }
            fn run(&self, ctx: &mut RenderContext) -> Result<(), RenderError> {
                self.0.set(ctx.surface_color);
                Ok(())
            }
        }

        let probe = Probe(Cell::new(None));
        let surface = SurfaceColor::new(1, 2, 3);
        StageCache::default()
            .render(&blank(8), Some(surface), &[&probe])
            .unwrap();

        assert_eq!(probe.0.get(), Some(surface));
    }

    // ---- RasterCustomizer over a folder base --------------------------------

    const TEST_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"##;

    /// Test surface color — uses the same golden-yellow as Windows folders.
    const TEST_SURFACE: SurfaceColor = SurfaceColor::new(255, 217, 112);

    fn solid(size: u32, rgba: [u8; 4]) -> IconImage {
        IconImage::new_full_content(RgbaImage::from_pixel(size, size, image::Rgba(rgba)), 1.0)
    }

    /// A 16px red and a 32px green icon over the test surface.
    fn folder_customizer() -> RasterCustomizer {
        let mut set = IconSet::new();
        set.add_image(solid(16, [255, 0, 0, 255]));
        set.add_image(solid(32, [0, 255, 0, 255]));
        RasterCustomizer::from_folder(FolderIconBase::new(set, TEST_SURFACE))
    }

    /// Catches a layer added to `Layers` but left out of `stages()` — it
    /// would never render, and its edits would never invalidate anything.
    #[test]
    fn every_layer_is_a_stage() {
        let versions = |c: &RasterCustomizer| -> Vec<u64> {
            RasterCustomizer::stages(&c.layers)
                .iter()
                .map(|s| s.version())
                .collect()
        };
        let baseline = versions(&folder_customizer());
        let placement = Placement::new(OverlayPosition::Center, OverlayAnchorMode::Inset, 0.5);

        let toggles: [fn(&mut Layers, Placement); 4] = [
            |l, _| {
                l.solid_color.set_config(Some(SolidColorConfig::new(1, 2, 3)));
            },
            |l, _| {
                l.decal.set_config(Some(DecalConfig::new("<svg/>", 0.5)));
            },
            |l, _| {
                l.color_dot.set_config(Some(ColorDotConfig::new(1, 2, 3)));
            },
            |l, p| {
                l.overlay.set_config(Some(ImageOverlayConfig::from_svg("<svg/>", p)));
            },
        ];

        for (i, toggle) in toggles.iter().enumerate() {
            let mut c = folder_customizer();
            toggle(&mut c.layers, placement);
            assert_ne!(versions(&c), baseline, "layer #{i} is not in stages()");
        }
    }

    #[test]
    fn base_kind_follows_the_base() {
        assert_eq!(folder_customizer().base_kind(), IconBaseKind::RasterFolder);
        assert_eq!(
            RasterCustomizer::from_icon_set(IconSet::new()).base_kind(),
            IconBaseKind::RasterCustomImage
        );
    }

    #[test]
    fn render_without_customizations() {
        let mut customizer = folder_customizer();
        let rendered = customizer.render_preview(16).unwrap();
        assert_eq!(rendered.dimensions().width, 16);
        assert_eq!(rendered.data.get_pixel(0, 0).0, [255, 0, 0, 255]);
    }

    #[test]
    fn render_all_sizes() {
        let mut customizer = folder_customizer();
        assert_eq!(customizer.render_output().unwrap().len(), 2);
    }

    #[test]
    fn missing_size_is_an_error() {
        let mut customizer = RasterCustomizer::from_icon_set(IconSet::new());
        assert!(matches!(
            customizer.render_preview(16),
            Err(RenderError::NoBaseIcon { logical_size: 16 })
        ));
    }

    #[test]
    fn color_dot_draws_into_bottom_right_without_recoloring() {
        let mut customizer = folder_customizer();
        customizer
            .layers
            .color_dot
            .set_config(Some(ColorDotConfig::new(0, 0, 255)));

        // The 32px base is solid green; only the dot area should turn blue.
        let rendered = customizer.render_preview(32).unwrap();
        assert_eq!(rendered.data.get_pixel(0, 0).0, [0, 255, 0, 255]);

        // Dot center: 32 - margin(1) - size(12)/2 ≈ 25.
        let dot = rendered.data.get_pixel(25, 25).0;
        assert!(
            dot[2] > dot[1],
            "expected a blue dot in the bottom-right, got {dot:?}"
        );
    }

    #[test]
    fn solid_color_shifts_hue() {
        let mut customizer = folder_customizer();
        customizer
            .layers
            .solid_color
            .set_config(Some(SolidColorConfig::new(0, 188, 212)));

        let pixel = customizer.render_preview(16).unwrap().data.get_pixel(0, 0).0;
        assert!(pixel[1] > pixel[0], "Green should be > Red after hue shift");
        assert!(pixel[1] > pixel[2], "Green should be > Blue after hue shift");
    }

    #[test]
    fn solid_color_toggle_restores_the_base_and_back() {
        let mut customizer = folder_customizer();
        let config = SolidColorConfig::new(0, 188, 212);

        customizer.layers.solid_color.set_config(Some(config));
        let shifted = customizer.render_preview(16).unwrap().data.get_pixel(0, 0).0;

        customizer.layers.solid_color.set_config(None);
        let disabled = customizer.render_preview(16).unwrap().data.get_pixel(0, 0).0;
        assert_eq!(disabled, [255, 0, 0, 255]);

        customizer.layers.solid_color.set_config(Some(config));
        let restored = customizer.render_preview(16).unwrap().data.get_pixel(0, 0).0;
        assert_eq!(restored, shifted);
    }

    #[test]
    fn changing_solid_color_changes_the_output() {
        let mut customizer = folder_customizer();

        customizer
            .layers
            .solid_color
            .set_config(Some(SolidColorConfig::new(76, 175, 80)));
        let first = customizer.render_preview(16).unwrap().data.get_pixel(0, 0).0;

        customizer
            .layers
            .solid_color
            .set_config(Some(SolidColorConfig::new(33, 150, 243)));
        let second = customizer.render_preview(16).unwrap().data.get_pixel(0, 0).0;

        assert_ne!(first, second);
    }

    #[test]
    fn decal_follows_solid_color_through_the_pipeline() {
        let mut set = IconSet::new();
        set.add_image(solid(16, [255, 0, 0, 255]));
        let mut customizer = RasterCustomizer::from_folder(FolderIconBase::new(set, TEST_SURFACE));

        let config = SolidColorConfig::new(0, 188, 212);
        customizer.layers.solid_color.set_config(Some(config));
        customizer
            .layers
            .decal
            .set_config(Some(DecalConfig::new(TEST_SVG, 0.5)));

        let with_ct = customizer.render_preview(16).unwrap().data.get_pixel(0, 0).0;
        assert!(with_ct[1] > with_ct[0], "recolored face should be green-dominant");

        // Dropping solid color must drop the stage the decal was built on.
        customizer.layers.solid_color.set_config(None);
        let without_ct = customizer.render_preview(16).unwrap().data.get_pixel(0, 0).0;
        assert_eq!(without_ct, [255, 0, 0, 255]);

        customizer.layers.solid_color.set_config(Some(config));
        let restored = customizer.render_preview(16).unwrap().data.get_pixel(0, 0).0;
        assert_eq!(restored, with_ct);
    }

    // ---- Layer-level behaviour that the pipeline relies on ------------------

    #[test]
    fn decal_tints_against_the_dominant_color_when_present() {
        use crate::layer::solid_color::apply_solid_color;
        use crate::layer::{DominantColor, RasterPaint};

        let config = SolidColorConfig::new(0, 188, 212);
        let mut ctx = RenderContext::new(solid(16, [255, 0, 0, 255]));
        ctx.surface_color = Some(TEST_SURFACE);
        apply_solid_color(&mut ctx.image, &TEST_SURFACE, &config);
        ctx.dominant_color = Some(DominantColor::new(
            config.target_r,
            config.target_g,
            config.target_b,
            255,
        ));
        let emitted = ctx.dominant_color.unwrap().as_tuple();

        DecalConfig::new(TEST_SVG, 0.5).paint(&mut ctx).unwrap();

        assert_eq!(
            ctx.dominant_color.unwrap().as_tuple(),
            emitted,
            "Decal should consume but not overwrite DominantColor"
        );
    }

    #[test]
    fn decal_falls_back_to_the_surface_color() {
        let ct_layer: Layer<SolidColorConfig> = Layer::default();
        let mut decal_layer: Layer<DecalConfig> = Layer::default();
        decal_layer.set_config(Some(DecalConfig::new(TEST_SVG, 0.5)));

        let mut ctx = RenderContext::new(solid(16, [0, 0, 255, 255]));
        ctx.surface_color = Some(TEST_SURFACE);

        ct_layer.run(&mut ctx).unwrap();
        assert!(ctx.dominant_color.is_none());
        assert_eq!(ctx.image.data.get_pixel(0, 0).0, [0, 0, 255, 255]);

        // The corner is outside the 8px centered decal; the center is inside.
        decal_layer.run(&mut ctx).unwrap();
        assert_eq!(ctx.image.data.get_pixel(0, 0).0, [0, 0, 255, 255]);
        assert_ne!(ctx.image.data.get_pixel(8, 8).0, [0, 0, 255, 255]);
    }

    #[test]
    fn darken_color_works() {
        let original = (200, 100, 100, 255);
        let darkened = darken_color(original, 0.2);

        let orig_brightness = (original.0 as u32 + original.1 as u32 + original.2 as u32) / 3;
        let dark_brightness = (darkened.0 as u32 + darkened.1 as u32 + darkened.2 as u32) / 3;
        assert!(dark_brightness < orig_brightness);
        assert_eq!(darkened.3, original.3);
    }

    #[test]
    fn decal_config_clamps_scale() {
        let config = DecalConfig::new("<svg></svg>", 0.5);
        assert_eq!(
            config.source,
            crate::layer::SvgSource::Raw("<svg></svg>".into())
        );
        assert_eq!(config.scale, 0.5);
        assert_eq!(DecalConfig::new("svg", 1.5).scale, 1.0);
    }
}
