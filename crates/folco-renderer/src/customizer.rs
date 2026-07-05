//! Generic icon customization engine.
//!
//! [`IconCustomizer<L>`] is a generic engine parameterized by a [`LayerSet`]
//! that defines which layers exist and how they are composed.
//!
//! Concrete customizer types are constructed as type aliases:
//! - [`FolderIconCustomizer`](crate::FolderIconCustomizer) = `IconCustomizer<FolderLayers>`
//! - [`CustomIconCustomizer`](crate::CustomIconCustomizer) = `IconCustomizer<OverlayLayers>`

use crate::icon::{IconBase, IconImage, IconSet, SurfaceColor};
use crate::layer::{CacheKey, CompositeLayer, DependencyVersion, RenderContext};
use crate::error::RenderError;

// ============================================================================
// LayerSet Trait
// ============================================================================

/// Trait for an ordered set of layers that can render into a context.
///
/// Implementors define which layers exist, their execution order,
/// and how tiles are composited. The generic [`IconCustomizer<L>`]
/// calls [`execute()`](LayerSet::execute) without knowing which layers
/// are involved.
pub trait LayerSet {
    /// Execute all layers against the render context in order.
    ///
    /// Each layer should either mutate `ctx.image` directly (base layers)
    /// or render a tile and composite it via `composite_over`.
    fn execute(&mut self, ctx: &mut RenderContext, key: CacheKey) -> Result<(), RenderError>;

    /// Combined version of all layers.
    ///
    /// Used by the composite cache to detect when any layer has changed.
    fn combined_version(&self) -> DependencyVersion;

    /// Invalidate all layer caches.
    fn invalidate_all(&mut self);
}

// ============================================================================
// IconCustomizer<L>
// ============================================================================

/// Generic icon customization engine.
///
/// Holds a base icon set (via [`IconBase`]) and an ordered set of layers
/// (via the [`LayerSet`] trait). The engine handles rendering, size lookup,
/// and composite caching — the layer set handles layer-specific logic.
///
/// Specialized customizer types are constructed as type aliases:
/// - [`FolderIconCustomizer`](crate::FolderIconCustomizer) = `IconCustomizer<FolderLayers>`
/// - [`CustomIconCustomizer`](crate::CustomIconCustomizer) = `IconCustomizer<OverlayLayers>`
///
/// # Example (folder icons)
///
/// ```
/// use folco_renderer::{FolderIconCustomizer, FolderIconBase, IconSet, FolderColorTargetConfig, DecalConfig, SurfaceColor};
///
/// let surface = SurfaceColor::new(255, 217, 112);
/// let base = FolderIconBase::new(IconSet::new(), surface);
/// let mut customizer = FolderIconCustomizer::from_folder(base);
///
/// customizer.layers.folder_color_target.set_config(Some(FolderColorTargetConfig::new(33, 150, 243)));
/// customizer.layers.decal.set_config(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
///
/// let output = customizer.render_all();
/// ```
///
/// # Example (custom icons)
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
pub struct IconCustomizer<L: LayerSet> {
    /// The base icons and associated metadata.
    base: IconBase,

    /// The layer set. Access layers directly to configure them.
    pub layers: L,

    /// Composite cache for final rendered images.
    composite: CompositeLayer,
}

impl<L: LayerSet> IconCustomizer<L> {
    /// Creates a new customizer with the given base and layer set.
    pub fn new(base: IconBase, layers: L) -> Self {
        Self {
            base,
            layers,
            composite: CompositeLayer::default(),
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
    pub fn render(&mut self, logical_size: u32) -> Result<IconImage, RenderError> {
        let base = self
            .base
            .icons()
            .find_by_logical_size(logical_size)
            .ok_or(RenderError::NoBaseIcon { logical_size })?
            .clone();
        self.render_icon(&base)
    }

    /// Renders all sizes in the base icon set with customizations applied.
    ///
    /// Returns a new `IconSet` containing the rendered images.
    ///
    /// # Errors
    ///
    /// Returns a render error if any layer fails.
    pub fn render_all(&mut self) -> Result<IconSet, RenderError> {
        let base_images: Vec<_> = self.base.icons().iter().cloned().collect();
        let mut rendered = Vec::with_capacity(base_images.len());
        for base in &base_images {
            rendered.push(self.render_icon(base)?);
        }
        Ok(IconSet::from_images(rendered))
    }

    /// Renders a single icon through the layer set with composite caching.
    fn render_icon(&mut self, base: &IconImage) -> Result<IconImage, RenderError> {
        let key = CacheKey::from_icon(base);
        let composite_deps = self.layers.combined_version();

        // Check composite cache first
        if let Some(cached) = self.composite.get_cached(key, composite_deps) {
            return Ok(cached.clone());
        }

        // Create render context; set surface color if folder-based
        let mut ctx = RenderContext::new(base.clone());
        if let Some(sc) = self.base.surface_color() {
            ctx.set(*sc);
        }

        // Execute the layer set
        self.layers.execute(&mut ctx, key)?;

        // Cache the final result
        self.composite.store(key, ctx.image.clone(), composite_deps);
        Ok(ctx.image)
    }

    /// Clears all layer caches and the composite cache.
    pub fn clear_cache(&mut self) {
        self.layers.invalidate_all();
        self.composite.invalidate();
    }
}
