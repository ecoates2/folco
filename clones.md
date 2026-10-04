# Unnecessary Clone Analysis

Comprehensive analysis of `.clone()` calls across the folco codebase, categorized by fix priority and impact.

---

## Category 1: Unnecessary Clones (Can Be Fixed)

### 1.1 `set_config` clones — 5 occurrences

**Files:** `crates/folco-renderer/src/folder_customizer.rs:165-167`, `custom_customizer.rs:120`, `svg_folder_customizer.rs:198`

```rust
// folder_customizer.rs:165-167
self.layers
    .solid_color
    .set_config(profile.solid_color.clone());  // <-- unnecessary clone
self.layers.decal.set_config(profile.decal.clone());  // <-- unnecessary clone
self.layers.overlay.set_config(profile.overlay.clone());  // <-- unnecessary clone

// custom_customizer.rs:120
self.layers.overlay.set_config(profile.overlay.clone());  // <-- unnecessary clone

// svg_folder_customizer.rs:198
self.layers.overlay.set_config(profile.overlay.clone());  // <-- unnecessary clone
```

**Problem:** `set_config` takes `Option<C>` by value, but the profile is only borrowed (`&FolderProfile`). The config is cloned once for the `differs_from` comparison, then cloned again for storage.

**Fix:** Change `set_config` to take `impl Into<Option<C>>` or `Option<impl FnOnce() -> C>` so the config is only cloned when actually stored (i.e., when it differs).

---

### 1.2 `RenderContext::new` owned parameter — 2 occurrences

**File:** `crates/folco-renderer/src/customizer.rs:140,171`

```rust
// customizer.rs:140 — in render()
let base = self
    .base
    .icons()
    .find_by_logical_size(logical_size)
    .ok_or(RenderError::NoBaseIcon { logical_size })?
    .clone();  // <-- clone to satisfy owned parameter
self.render_icon(&base)

// customizer.rs:171 — in render_icon()
let mut ctx = RenderContext::new(base.clone());  // <-- clone again
```

**Problem:** `render()` clones the base icon to pass to `render_icon()`, which clones again for `RenderContext::new()`.

**Fix:** Make `RenderContext::new` take `&IconImage` instead of owned `IconImage`.

---

### 1.3 Cache store clones — 8 occurrences

**Files:** `solid_color.rs:105,130`, `decal.rs:75,118`, `overlay.rs:189,195`, `color_dot.rs:112,118`, `svg_folder_customizer.rs:187`

```rust
// solid_color.rs:105
ctx.image = img.clone();  // clone to set context image

// solid_color.rs:130
self.store(key, CachedOutput::Image(ctx.image.clone()), deps);  // clone for cache

// decal.rs:75
return Ok(Some(tile.clone()));  // clone for early return

// decal.rs:118
self.store(key, CachedOutput::Tile(tile.clone()), deps);  // clone for cache

// overlay.rs:189
return Ok(Some(tile.clone()));  // clone for early return

// overlay.rs:195
self.store(key, CachedOutput::Tile(tile.clone()), deps);  // clone for cache

// color_dot.rs:112
return Ok(Some(tile.clone()));  // clone for early return

// color_dot.rs:118
self.store(key, CachedOutput::Tile(tile.clone()), deps);  // clone for cache

// svg_folder_customizer.rs:187
self.preview.store(key, image.clone(), deps);  // clone for cache
```

**Problem:** Values are cloned to populate caches, preventing the original from being moved. The cache could take ownership on first insert.

**Fix:** Use `Option::take()` or a `store_or_clone` method that takes ownership when the cache entry doesn't exist yet.

---

### 1.4 `FolderStrategy` clones — 3 occurrences

**File:** `crates/folco-core/src/context.rs:208,275,297`

### 1.5 `From<&CustomizationProfile>` conversion clones — 4 occurrences

**File:** `crates/folco-renderer/src/profile.rs:236-239,305`

```rust
// profile.rs:236-239 — From<&CustomizationProfile> for FolderProfile
Self {
    solid_color: profile.solid_color.clone(),  // <-- clone
    color_dot: profile.color_dot,              // ok: ColorDotConfig is Copy
    decal: profile.decal.clone(),              // <-- clone
    overlay: profile.overlay.clone(),          // <-- clone
}

// profile.rs:305 — From<&CustomizationProfile> for CustomProfile
Self {
    color_dot: profile.color_dot,              // ok: Copy
    overlay: profile.overlay.clone(),          // <-- clone
}
```

**Problem:** Converting from a borrowed `&CustomizationProfile` requires cloning all `Option<Config>` fields that contain heap-allocated data (`DecalConfig` has a `String`, `ImageOverlayConfig` has `ImageSource` which may contain `Vec<u8>`).

**Fix:** Accept owned `CustomizationProfile` in the `From` impl, or use `Arc`-backed config wrappers so clones are cheap pointer copies.

```rust
// context.rs:208
let base = SvgFolderIconBase::new(svg.clone(), crate::sys::SURFACE_COLOR);

// context.rs:275
FolderIconBase::new(
    c.base_icons().clone(),  // clones entire IconSet
    *c.surface_color()
        .expect("raster folder customizer always has a surface color"),
)

// context.rs:297
FolderStrategy::Svg(c) => Some(c.base().svg.clone()),
```

**Problem:** `svg.clone()` clones a String to move into a new struct. `c.base_icons().clone()` clones the entire icon set. `c.base().svg.clone()` returns a String clone for the caller.

**Fix:** Use `Arc<String>` for SVG data, or restructure `FolderStrategy` to expose references.

---

## Category 2: Unavoidable Clones (API Design)

### 2.1 `filter()` — owned return from borrowed input

**File:** `crates/folco-renderer/src/capabilities.rs:218-228`

```rust
pub fn filter(&self, profile: &CustomizationProfile) -> CustomizationProfile {
    CustomizationProfile {
        solid_color: self.solid_color.then(|| profile.solid_color.clone()).flatten(),
        color_dot: self.color_dot.then_some(profile.color_dot).flatten(),
        decal: self.decal.then(|| profile.decal.clone()).flatten(),
        overlay: self.overlay.then(|| profile.overlay.clone()).flatten(),
    }
}
```

**Problem:** Takes `&CustomizationProfile` but returns owned `CustomizationProfile`. Clones `DecalConfig` and `ImageOverlayConfig` which contain `String`/`Vec` fields.

**Impact:** Low — only called in tests (line 292).

**Fix:** Accept owned profile via `FnOnce` or accept `&CustomizationProfile` and return `CustomizationProfile` with `Arc`-backed configs.

---

### 2.2 `set_config` ownership — inherent API limitation

**File:** `crates/folco-renderer/src/layer/mod.rs:284`, `svg_layer/mod.rs:59`

```rust
pub fn set_config(&mut self, config: Option<C>) -> bool {
```

**Problem:** Takes ownership of `C`, but callers often only have `&C`. The `differs_from` comparison borrows both, then the new config is moved in.

**Fix:** See Category 1.1 — change signature to avoid redundant clones.

---

## Category 3: Minor Clones (Low Impact)

### 3.1 `Copy` type clones — no-op

**File:** `crates/folco-cli/src/main.rs:330,371`

```rust
overlay_anchor_mode.clone().into()
```

**Impact:** Negligible — `OverlayAnchorMode` is `Copy`, so `.clone()` is effectively a no-op.

---

### 3.2 Small String clones in convert.rs

**Files:** `crates/folco-core/src/convert.rs:552,664,845`

```rust
path: path.clone()
```

**Impact:** Small — these are `Path` clones for struct construction, low cost.

### 3.3 Error-path clones in svg.rs

**File:** `crates/folco-renderer/src/layer/svg.rs:128,138`

```rust
// svg.rs:128 — emoji clone for error
let asset = resolve_twemoji(emoji).ok_or_else(|| RenderError::InvalidEmoji {
    emoji: emoji.clone(),  // <-- clone only on error path
})?;

// svg.rs:138 — name clone for error
let asset = SvgTwemojiAsset::from_name(name)
    .ok_or_else(|| RenderError::InvalidEmojiName { name: name.clone() })?;
```

**Impact:** Negligible — clones only occur when an error is raised, not on the happy path.

### 3.4 `from_rgba_image` clone in image_source.rs

**File:** `crates/folco-renderer/src/layer/image_source.rs:85`

```rust
pub fn from_rgba_image(img: &RgbaImage) -> Result<Self, RenderError> {
    Self::from_dynamic_image(&image::DynamicImage::ImageRgba8(img.clone()))
}
```

**Impact:** Low — clones an `RgbaImage` (typically 64-256KB) but only for PNG encoding, not for rendering. The clone is inherent since `from_dynamic_image` takes ownership.

---

### 3.3 Test code clones

**Files:** `crates/folco-renderer/src/folder_customizer.rs:515,566,621,648,689`, `crates/folco-renderer/src/svg_folder_customizer.rs:180`

**Impact:** Negligible — test code, performance doesn't matter.

---

## Summary

| Category | Count | Impact | Effort |
|----------|-------|--------|--------|
| 1. Unnecessary (fixable) | ~16 | Medium | Low-Medium |
| 2. Unavoidable (API design) | ~5 | Low | Medium |
| 3. Minor (low impact) | ~10 | Negligible | Trivial |

**Highest ROI fixes:**
1. Change `set_config` to avoid redundant clones (Category 1.1)
2. Make `RenderContext::new` take `&IconImage` (Category 1.2)
3. Use `Option::take()` pattern in cache stores (Category 1.3)
