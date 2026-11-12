//! Generic icon customization engine.
//!
//! [`IconCustomizer<L>`] is a generic engine parameterized by a [`LayerSet`]
//! that lists which layers exist and in what order. The engine runs them as
//! stages and caches the [`RenderContext`] after each one, per size, so a
//! change to layer *k* re-runs only stages *k* and later.
//!
//! Concrete customizer types are constructed as type aliases:
//! - [`FolderIconCustomizer`](crate::FolderIconCustomizer) = `IconCustomizer<FolderLayers>`
//! - [`CustomIconCustomizer`](crate::CustomIconCustomizer) = `IconCustomizer<CustomLayers>`

use std::collections::HashMap;

use crate::Render;
use crate::error::RenderError;
use crate::icon::{IconBase, IconImage, IconSet, SurfaceColor};
use crate::layer::{CacheKey, RenderContext, Stage};

// ============================================================================
// LayerSet Trait
// ============================================================================

/// An ordered set of raster layers.
///
/// Implementors own the concrete `Layer<C>` fields and hand them back as
/// [`Stage`]s in execution order. That single list is the pipeline: the engine
/// runs it, caches after each entry, and compares versions against it. A layer
/// missing from the list simply never renders, which no test can miss.
pub trait LayerSet {
    /// The layers to run, in order. Later stages draw over earlier ones.
    fn stages(&self) -> Vec<&dyn Stage>;

    /// Current version of every stage, in order.
    fn versions(&self) -> Vec<u64> {
        self.stages().iter().map(|s| s.version()).collect()
    }
}

// ============================================================================
// StageCache
// ============================================================================

/// Context snapshots after each stage, per base size.
///
/// `snapshots[i]` is the context after stage `i` ran at `versions[i]`. A
/// render reuses the longest prefix whose versions still match and rebuilds
/// from there, so editing the last layer costs exactly one stage.
#[derive(Default)]
struct StageCache {
    by_size: HashMap<CacheKey, Vec<Snapshot>>,
}

struct Snapshot {
    version: u64,
    ctx: RenderContext,
}

impl StageCache {
    /// Renders `base` through `stages`, reusing cached prefixes where valid.
    fn render(
        &mut self,
        base: &IconImage,
        surface_color: Option<SurfaceColor>,
        stages: &[&dyn Stage],
    ) -> Result<IconImage, RenderError> {
        let snapshots = self.by_size.entry(CacheKey::from_icon(base)).or_default();

        let valid = snapshots
            .iter()
            .zip(stages)
            .take_while(|(snap, stage)| snap.version == stage.version())
            .count();
        snapshots.truncate(valid);

        if valid == stages.len() {
            return Ok(match snapshots.last() {
                Some(snap) => snap.ctx.image.clone(),
                None => base.clone(),
            });
        }

        let mut ctx = match snapshots.last() {
            Some(snap) => snap.ctx.clone(),
            None => RenderContext {
                image: base.clone(),
                surface_color,
                dominant_color: None,
            },
        };

        for stage in &stages[valid..] {
            stage.run(&mut ctx)?;
            snapshots.push(Snapshot {
                version: stage.version(),
                ctx: ctx.clone(),
            });
        }

        Ok(ctx.image)
    }
}

// ============================================================================
// IconCustomizer<L>
// ============================================================================

/// Generic icon customization engine.
///
/// Holds a base icon set (via [`IconBase`]) and an ordered set of layers
/// (via the [`LayerSet`] trait). The engine handles rendering, size lookup,
/// and per-stage caching — the layer set handles layer-specific logic.
///
/// Specialized customizer types are constructed as type aliases:
/// - [`FolderIconCustomizer`](crate::FolderIconCustomizer) = `IconCustomizer<FolderLayers>`
/// - [`CustomIconCustomizer`](crate::CustomIconCustomizer) = `IconCustomizer<CustomLayers>`
///
/// # Example (folder icons)
///
/// ```
/// use folco_renderer::{FolderIconCustomizer, FolderIconBase, IconSet, SolidColorConfig, DecalConfig, SurfaceColor, Render};
///
/// let surface = SurfaceColor::new(255, 217, 112);
/// let base = FolderIconBase::new(IconSet::new(), surface);
/// let mut customizer = FolderIconCustomizer::from_folder(base);
///
/// customizer.layers.solid_color.set_config(Some(SolidColorConfig::new(33, 150, 243)));
/// customizer.layers.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
///
/// let output = customizer.render_full_output();
/// ```
///
/// # Example (custom icons)
///
/// ```ignore
/// use folco_renderer::{CustomIconCustomizer, ImageSource, IconSizeSpec, ImageOverlayConfig, OverlayAnchorMode, OverlayPosition, Placement, Render};
///
/// let source = ImageSource::svg("<svg>...</svg>");
/// let specs = vec![IconSizeSpec::square(32, 1.0), IconSizeSpec::square(256, 1.0)];
/// let mut customizer = CustomIconCustomizer::from_image(&source, &specs).unwrap();
///
/// customizer.layers.overlay.set_config(Some(ImageOverlayConfig::from_svg(
///     "<svg>badge</svg>",
///     Placement::new(OverlayPosition::BottomRight, OverlayAnchorMode::Inset, 0.25),
/// )));
///
/// let output = customizer.render_full_output();
/// ```
pub struct IconCustomizer<L: LayerSet> {
    /// The base icons and associated metadata.
    base: IconBase,

    /// The layer set. Access layers directly to configure them.
    pub layers: L,

    cache: StageCache,
}

impl<L: LayerSet> IconCustomizer<L> {
    /// Creates a new customizer with the given base and layer set.
    pub fn new(base: IconBase, layers: L) -> Self {
        Self {
            base,
            layers,
            cache: StageCache::default(),
        }
    }

    // ---- Accessors ----------------------------------------------------------

    /// Returns a reference to the icon base (enum discriminant reveals the variant).
    pub fn base(&self) -> &IconBase {
        &self.base
    }

    /// Returns a reference to the base icon set.
    pub fn base_icons(&self) -> &IconSet {
        self.base.icons()
    }

    /// Returns the surface color, if this is a folder-based customizer.
    pub fn surface_color(&self) -> Option<&SurfaceColor> {
        self.base.surface_color()
    }

    // ---- Rendering ----------------------------------------------------------

    /// Renders a single icon at the specified logical size.
    ///
    /// Returns the closest matching size from the base icon set,
    /// with all applicable customizations applied.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::NoBaseIcon`] if no base icon matches the size,
    /// or a render error if a layer fails.
    fn render(&mut self, logical_size: u32) -> Result<IconImage, RenderError> {
        let icon = self
            .base
            .icons()
            .find_by_logical_size(logical_size)
            .ok_or(RenderError::NoBaseIcon { logical_size })?;
        self.cache
            .render(icon, self.base.surface_color().copied(), &self.layers.stages())
    }

    /// Renders all sizes in the base icon set with customizations applied.
    ///
    /// Returns a new `IconSet` containing the rendered images.
    ///
    /// # Errors
    ///
    /// Returns a render error if any layer fails.
    fn render_all(&mut self) -> Result<IconSet, RenderError> {
        let surface = self.base.surface_color().copied();
        let stages = self.layers.stages();
        let rendered = self
            .base
            .icons()
            .iter()
            .map(|icon| self.cache.render(icon, surface, &stages))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(IconSet::from_images(rendered))
    }

    /// Drops every cached stage snapshot.
    pub fn clear_cache(&mut self) {
        self.cache.by_size.clear();
    }
}

impl<L: LayerSet> Render for IconCustomizer<L> {
    /// Rasterized icon images
    type Output = IconSet;
    fn render_full_output(&mut self) -> Result<Self::Output, RenderError> {
        self.render_all()
    }
    fn render_raster_preview(&mut self, size: u32) -> Result<IconImage, RenderError> {
        self.render(size)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::RectPx;
    use image::RgbaImage;
    use std::cell::Cell;

    /// A stage that records how often it ran and paints its id into pixel (0, 0).
    struct Counting {
        id: u8,
        version: Cell<u64>,
        runs: Cell<usize>,
    }

    impl Counting {
        fn new(id: u8) -> Self {
            Self {
                id,
                version: Cell::new(0),
                runs: Cell::new(0),
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

        fn run(&self, ctx: &mut RenderContext) -> Result<(), RenderError> {
            self.runs.set(self.runs.get() + 1);
            let px = ctx.image.data.get_pixel_mut(0, 0);
            px.0[self.id as usize] = px.0[self.id as usize].wrapping_add(1);
            Ok(())
        }
    }

    fn base(size: u32) -> IconImage {
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

        let first = cache.render(&base(8), None, &refs).unwrap();
        let second = cache.render(&base(8), None, &refs).unwrap();

        assert_eq!(runs(&stages), [1, 1, 1]);
        assert_eq!(first.data.as_raw(), second.data.as_raw());
        // Each stage painted exactly once into the cached image.
        assert_eq!(first.data.get_pixel(0, 0).0, [1, 1, 1, 0]);
    }

    #[test]
    fn changing_a_stage_reruns_only_it_and_later_stages() {
        let stages = [Counting::new(0), Counting::new(1), Counting::new(2)];
        let refs: Vec<&dyn Stage> = stages.iter().map(|s| s as &dyn Stage).collect();
        let mut cache = StageCache::default();

        cache.render(&base(8), None, &refs).unwrap();
        stages[1].bump();
        let image = cache.render(&base(8), None, &refs).unwrap();

        assert_eq!(runs(&stages), [1, 2, 2]);
        // Stage 0's output was reused, not re-painted on top of itself.
        assert_eq!(image.data.get_pixel(0, 0).0, [1, 1, 1, 0]);
    }

    #[test]
    fn changing_the_last_stage_costs_one_stage() {
        let stages = [Counting::new(0), Counting::new(1), Counting::new(2)];
        let refs: Vec<&dyn Stage> = stages.iter().map(|s| s as &dyn Stage).collect();
        let mut cache = StageCache::default();

        cache.render(&base(8), None, &refs).unwrap();
        stages[2].bump();
        cache.render(&base(8), None, &refs).unwrap();

        assert_eq!(runs(&stages), [1, 1, 2]);
    }

    #[test]
    fn sizes_are_cached_independently() {
        let stages = [Counting::new(0)];
        let refs: Vec<&dyn Stage> = stages.iter().map(|s| s as &dyn Stage).collect();
        let mut cache = StageCache::default();

        cache.render(&base(8), None, &refs).unwrap();
        cache.render(&base(16), None, &refs).unwrap();
        cache.render(&base(8), None, &refs).unwrap();

        assert_eq!(runs(&stages), [2]);
    }

    #[test]
    fn empty_pipeline_returns_the_base() {
        let mut cache = StageCache::default();
        let image = cache.render(&base(8), None, &[]).unwrap();
        assert_eq!(image.data.as_raw(), base(8).data.as_raw());
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
            .render(&base(8), Some(surface), &[&probe])
            .unwrap();

        assert_eq!(probe.0.get(), Some(surface));
    }
}
