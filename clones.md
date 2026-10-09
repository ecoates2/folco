# Unnecessary Clone Analysis

Covers `.clone()`, `.cloned()`, `.to_vec()`, `.to_string()` on already-owned data, `to_rgba8()` on owned images, and hidden copies through `impl Into<String>` taking a `&String`. Scope is `crates/` and `gui/src-tauri/src/`. Line numbers were checked on 2026-10-08.

**Hot paths, used to rank the work:**
- **Live preview (wasm):** `CanvasRenderer::render_preview` → `IconCustomizer::render` / `SvgFolderIconCustomizer::render_raster_preview`. Runs on every edit.
- **Apply (CLI/core):** `CustomizationContext::apply_profile` → `FolderStrategy::render_output` → `to_sys`.
- **Startup / IPC:** `FolderStrategy::from_sys_icon_set`, icon cache, `folder_icon_base`.

---

## Category 1: Avoidable, worth fixing

### 1.1 Profile application clones each heap config twice — ✅ DONE

> **Completed 2026-10-08.** All three `apply_profile` methods take their profile by value and move each config into `set_config`. Added `From<CustomizationProfile>` (by value) for `FolderProfile` and `CustomProfile`, and kept `From<&_>` for callers that need to hold on to the profile. `CustomizationContext::apply_profile`, `customize_folder`, `customize_folders` and `customize_folders_async` now take `CustomizationProfile` by value. The CLI and wasm `import_profile_json` move their owned profiles through, so neither path clones. `SolidColorConfig` now derives `Copy`. `set_config_ref` wasn't needed: no non-test caller is left with only a borrow.

The two halves were listed separately in the previous version. They stack:

```
CustomizationProfile ──From<&_> (clone #1)──▶ FolderProfile / CustomProfile ──apply_profile(&_) (clone #2)──▶ set_config(Option<C>)
```

| Clone | Sites |
|---|---|
| #1 | `folco-renderer/src/profile.rs:236-239`, `:305` (`From<&CustomizationProfile>`) |
| #2 | `folder_customizer.rs:165-168`, `custom_customizer.rs:120`, `svg_folder_customizer.rs:198` |
| Callers that own the profile but borrow it anyway | `folco-core/src/context.rs:224` (`profile.into()`), `folco-renderer-wasm/src/canvas.rs:639,643,647` (`(&wire).into()`; `wire` is dropped right afterwards), `folco-cli/src/main.rs:506` (`(&profile).into()`; `profile` is owned) |

- **Correction:** `set_config` does not clone. The waste is that the caller clones before anyone knows whether the config changed, and when it hasn't, the clone is dropped.
- `ImageOverlayConfig` holding `ImageSource::Raster(Vec<u8>)` contains a full PNG, so it's the expensive one. `DecalConfig` holds an SVG `String`.
- `SolidColorConfig` is three `u8` fields. Derive `Copy` like `ColorDotConfig` so `.clone()` / `.cloned()` stop looking like heap clones.

**Fix:**
- Make `apply_profile` take `FolderProfile` / `CustomProfile` **by value** and move each field into `set_config`.
- Add `From<CustomizationProfile>` (by value) for both profile types.
- For callers that really only hold a borrow, add `set_config_ref(Option<&C>)` to `Layer` and `SvgLayer`. It compares first and clones only if the config differs.
- `CustomizationContext::apply_profile` / `customize_folders(_async)` take `&CustomizationProfile`, but the CLI owns its profile. Either take it by value or accept one clone there (one instead of two).

### 1.2 Base images cloned to get around the borrow checker (`customizer.rs:140`, `:152`) — ✅ DONE

> **Completed 2026-10-08.** `render_icon` is now an associated fn taking `(&mut L, &mut CompositeLayer, Option<&SurfaceColor>, &IconImage)`. `render` and `render_all` destructure `self` and pass the base icons by reference, so the only base copy left is the required one into `RenderContext`.

- `render()` clones the base `IconImage` (140), and `render_icon` then clones it again into the context (171). That's two full-image copies per preview frame, and one of them is wasted.
- `render_all()` clones **every** base image (152), only so it can call a `&mut self` method while iterating `self.base`. **This was missing from the previous version.**
- **Root cause:** `render_icon` takes `&mut self`, but it only mutates `layers` and `composite` and only reads `base`.
- **Fix:** split borrows. Make `render_icon` an associated fn taking `(&mut L, &mut CompositeLayer, Option<&SurfaceColor>, &IconImage)`, then destructure `self` in `render` / `render_all`.
- The clone at 171 stays: the context needs an owned image it can mutate. The earlier idea of `RenderContext::new(&IconImage)` would only move that clone inside the constructor.

### 1.3 Tile layers clone on cache hit *and* on store (6 sites) — ✅ DONE

> **Completed 2026-10-08.** `render_tile` now returns `Option<&RgbaImage>` via the shared `Layer::cached_tile` helper (`layer/mod.rs`), so tiles are never cloned. Validity is checked first, then the tile is rendered and stored on a miss, then read back once to get around the borrowck issue below.

`layer/color_dot.rs:112,118`, `layer/decal.rs:75,81` (previously listed as `:118`), `layer/overlay.rs:189,195`.

- The only consumers are `folder_customizer.rs:89-108` and `custom_customizer.rs:40-51`, and they only call `composite_over(&mut ctx.image.data, &tile, ..)`.
- **Fix:** have `render_tile` return `Result<Option<&RgbaImage>, _>`. Store the freshly rendered tile and return a reference into the cache. That leaves zero clones.
- **Borrowck gotcha:** `if let Some(t) = self.get_cached(..) { return Ok(Some(t)); } self.store(..)` won't compile (NLL "problem case #3"). Check validity first (`bool`), render and store on a miss, then do a single `self.cache.get(&key)` at the end. The entry API works too.
- The previous `Option::take()` / `store_or_clone` suggestion would empty the cache, so it doesn't work here.

### 1.4 Solid color copies the image it's about to replace (`layer/solid_color.rs:121` → `:189`) — ✅ DONE

> **Completed 2026-10-08.** `apply_solid_color` now takes `&mut IconImage` and recolors `ctx.image` in place. `SurfaceColor` is copied out of the context first so the image can be borrowed mutably. The test at `folder_customizer.rs:515` is updated.

- `ctx.image = apply_solid_color(&ctx.image, ..)` clones `icon.data` (189), recolors the copy, then overwrites `ctx.image`.
- **Fix:** have `apply_solid_color` take `&mut IconImage` and recolor in place. That saves one full-image copy per cache miss. The test at `folder_customizer.rs:517` needs updating.

### 1.5 `to_sys()` clones render output that's dropped right after (`folco-core/src/convert.rs:27`, `:39`) — ✅ DONE

> **Completed 2026-10-08.** `SystemFormat::to_sys(&self)` became `into_sys(self)`. The raster set moves each `RgbaImage` into `DynamicImage::ImageRgba8`, and the SVG output moves its `String`. All four callers in `context.rs` and the tests are updated. The trait is crate-private, so no public API changed.

- `IconSet::to_sys(&self)` clones every rendered `RgbaImage`, and `SvgRenderOutput::to_sys(&self)` clones the SVG `String`.
- Every caller passes a temporary or a local that's never used again: `context.rs:262`, `:266`, `:754`, `:832`.
- **Fix:** consume it with `into_sys(self)`, using `DynamicImage::ImageRgba8(img.data)`.

### 1.6 `folder_icon_base()` clones the whole base `IconSet` for a caller that only borrows it (`context.rs:275`) — ✅ DONE

> **Completed 2026-10-08.** `CustomizationContext::folder_icon_base()` now returns `Option<&FolderIconBase>`, borrowed straight from `IconBase::Folder`. In the GUI, `AppState::get_folder_icon_base` builds the DTO inside `with_ctx` and returns `Option<FolderIconBaseDto>`, and the Tauri command just forwards it. Trade-off: PNG encoding now runs while the context mutex is held, which is fine for this startup-time call.

- It builds a new `FolderIconBase` from `c.base_icons().clone()`, but `IconBase::Folder(FolderIconBase)` already holds one.
- The only consumer is `gui/src-tauri/src/state.rs:69` → `lib.rs:24`, and it only needs `FolderIconBaseDto::try_from(&base)`.
- **Fix:** return `Option<&FolderIconBase>` by matching on `c.base()`, and convert to the DTO inside the `with_ctx` closure.

### 1.7 Startup: the system icon set is copied instead of moved — ✅ DONE

> **Completed 2026-10-08.** `FolderStrategy::from_sys_icon_set` takes `SysIconSet` by value and moves the SVG. A new crate-private `into_renderer_icon_set(SysIconSet)` uses `into_rgba8()` and is called by `from_sys_icon_set` and `IconCache::get_renderer_icon_set`. The public borrowing `convert_icon_set` is kept, and both versions share a `to_renderer_image` helper. `fetch_and_cache` saves the `DynamicImage` directly instead of converting it to RGBA first.

- `context.rs:208`: `from_sys_icon_set(&SysIconSet)` clones the SVG. Both callers (`context.rs:143-146`, `:585-586`) own the set and drop it afterwards. Take it by value and move the SVG.
- `convert.rs:74`: `convert_icon_set(&SysIconSet)` calls `to_rgba8()`, which always allocates. With ownership, `into_rgba8()` costs nothing when the image is already RGBA8. `convert_icon_set` is re-exported (`lib.rs:44`), so add a consuming variant rather than changing the signature.
- `cache.rs:166`: `image.data.to_rgba8()` exists only to read the width and save the image. `DynamicImage` has `width()` and `save()` itself. The reload path normalises through `convert_icon_set` anyway.

### 1.8 `to_rgba8()` on an owned `DynamicImage` → `into_rgba8()` — ✅ DONE

> **Completed 2026-10-08.** Switched to `into_rgba8()` in `folco-transfer/src/lib.rs`, `canvas.rs` (`from_png`, `from_png_multiple`) and `image_source.rs`. `render_at_size` now checks dimensions on the decoded image before converting, so the resize path no longer throws away a full RGBA copy. The borrowed `to_rgba8()` calls in `convert.rs` and `cache.rs` belong to 1.7.

- `folco-transfer/src/lib.rs:105`, `folco-renderer-wasm/src/canvas.rs:225`, `:268`, `layer/image_source.rs:122` (`resized.to_rgba8()`).
- `layer/image_source.rs:112`: the whole decoded image is converted to RGBA *before* checking whether a resize is needed, and that copy is thrown away when it is. Compare `img.width().max(img.height())` first, then use `img.into_rgba8()` or `img.resize(..).into_rgba8()`. This runs on every raster overlay tile miss and on every custom-icon size.

### 1.9 Copies of SVG strings — ✅ DONE

> **Completed 2026-10-08.** `SvgCanvas<'a>` now borrows `base: &'a str`, and `into_svg` builds the output in one allocation (it no longer `concat`s the overlays into an intermediate string). `Medium::Canvas` became a GAT so `SvgMedium` can name `SvgCanvas<'a>`. `render_svg_with_color` uses `Cow<str>` and only allocates when recoloring. `replace_svg_colors` feeds the input straight into the first `replace_color_attr` pass.

- `svg_folder_customizer.rs:157`: `SvgCanvas::new(&self.base.svg)` goes through `impl Into<String>`, which copies the whole base SVG on every `render_output` (every SVG preview miss and every apply). Make `SvgCanvas` borrow (`base: &'a str`) and allocate once in `into_svg`.
- `layer/svg.rs:203`: `svg_data.to_string()` when there's no fill color, only to hand `&svg_data` to `Tree::from_str`. Use `Cow<str>`. This runs on every SVG, emoji and SVG-preview rasterization.
- `layer/svg.rs:267`: `let mut result = svg_data.to_string()` is overwritten straight away by `replace_color_attr`, so the allocation is wasted. Called on every decal render.
- Together: one SVG folder preview miss copies the base SVG twice before it's rasterized.

### 1.10 `ImageSource::from_rgba_image` (`layer/image_source.rs:85`). Low priority.

- **Correction:** this clone isn't required. `RgbaImage::write_to` exists (`IconImage::to_png_bytes` already uses it), so encode directly. It's only called from tests at the moment.

### 1.11 `IconCapabilities::filter` (`capabilities.rs:223-227`). Low priority.

- **Correction:** this isn't unavoidable. Take `CustomizationProfile` by value and filter its fields. The only caller is a test (`:292`), so you could also just delete it.

### 1.12 CLI `overlay_anchor_mode.clone()` (`folco-cli/src/main.rs:330`, `:371`). Trivial.

- **Correction:** the value is `AnchorModeArg`, which is `Clone` but not `Copy`; `OverlayAnchorMode` isn't involved. The two branches are mutually exclusive, so it can be moved, the same way `overlay_position` already is. Remove `.clone()`.

---

## Category 2: Unavoidable with the current API (leave as is, or change the API)

### 2.1 Cache keeps a copy while an owned image is returned

`layer/solid_color.rs:105`, `:130`; `customizer.rs:167`, `:180`; `svg_folder_customizer.rs:180` (production code; the previous version listed it as a test), `:187`.

- `solid_color.rs:105`: a cache hit has to copy into `ctx`, because later layers composite onto `ctx.image` in place.
- The rest exist because `Render::render_raster_preview` returns an owned `IconImage`. The wasm consumers (`draw_rgba_to_canvas`, `render_to_pixels`) only need the bytes by reference, so returning `&IconImage` would remove the cache-hit clones (167, 180) and the store clone (187). Storing `Rc`/`Arc<IconImage>` would also work. Medium effort, and only worth it if preview cache hits are frequent.
- `render_full_output` collects into an owned `IconSet`, so the composite store at 180 stays for that path either way.

### 2.2 `export_profile()` clones its configs

`folder_customizer.rs:174-177`, `custom_customizer.rs:139`, `svg_folder_customizer.rs:217`.

- These are needed to return an owned `CustomizationProfile`. In practice wasm `export_profile_json` (`canvas.rs:618`) clones only to serialize. A borrowed serialization view would avoid copying a raster overlay. Low priority.

### 2.3 `base_svg()` returns `String` (`context.rs:297`)

- The consumer is a Tauri IPC DTO, which has to own its data once the mutex is released. One copy per call is unavoidable unless you switch to `Arc<str>`. Leave it.

### 2.4 `PathBuf` clones for progress events (`context.rs:552`, `:664`, `:845`)

- **Correction:** these are in `context.rs`, not `convert.rs`, and they're `PathBuf`s, not `String`s. Each folder sends two events that both own the path. Leave them.

### 2.5 Error-path clones (`layer/svg.rs:128`, `:138`)

- They only run when an error is raised. Leave them.

### 2.6 Copies at the wasm boundary

- `canvas.rs:265`: `png_data.to_vec()` copies from the JS heap into wasm memory, which is required. `canvas.rs:577`: the `copy_from` into a JS `Uint8Array` is also required.

### 2.7 One-off and DTO clones

- `gui/src-tauri/src/lib.rs:65`: the window config is cloned so it can be mutated. Runs once.
- `gui/src-tauri/src/lib.rs:74`: `AppHandle` clone, which is an `Arc` bump.
- `folco-model/src/folder_color.rs:143` and `capabilities.rs:134-137`: `&'static str` → `String` for serde DTOs.
- `to_string_lossy().to_string()` at `main.rs:454`, `:537`, `:597` and `cache.rs:175`, `:183`: use `.into_owned()` instead. Trivial.

### 2.8 Test code

- `folder_customizer.rs:515`, `:566`, `:621`, `:648`, `:689`. Leave them.

---

## Summary

| # | Item | Path | Per-call cost | Effort |
|---|------|------|---------------|--------|
| 1.3 | ✅ Tile clones (6) | Preview + apply | 2 full-size RGBA copies per tile layer | Done |
| 1.2 | ✅ Base image clones | Preview + apply | 1 image per preview; N images per apply | Done |
| 1.1 | ✅ Profile double clone | Apply / import | 2× each heap config (raster overlay = whole PNG) | Done |
| 1.4 | ✅ Solid color in place | Preview + apply | 1 image per miss | Done |
| 1.9 | ✅ SVG string copies | SVG preview / decal | 1–2 SVG copies per render | Done |
| 1.8 | ✅ `into_rgba8` / resize order | Overlay / custom / load | 1 decoded image | Done |
| 1.5 | ✅ `into_sys` | Apply | N rendered images / SVG | Done |
| 1.6 | ✅ `folder_icon_base` borrow | GUI IPC | Full base `IconSet` | Done |
| 1.7 | ✅ Consume `SysIconSet` | Startup | SVG + N images | Done |
| 1.10–1.12 | Misc. | Tests / CLI | Negligible | Trivial |

**Suggested order:** ~~1.3~~ → ~~1.2~~ → ~~1.4~~ (all inside the preview loop, mostly mechanical), then ~~1.1~~ (API change), then ~~1.9~~ and ~~1.5~~, ~~1.6~~, ~~1.7~~, ~~1.8~~, then the trivial ones.

### Corrections to the previous version
- 1.1: `set_config` doesn't clone twice. The clone is in the caller, and it compounds with the `From<&_>` clone.
- 1.2: `RenderContext::new(&IconImage)` doesn't remove a clone. The cause is a borrow conflict, and `render_all` (152) was missing.
- 1.3: `Option::take()` would empty the cache. Return references instead. `decal.rs:118` is actually `:81`. `solid_color.rs:105` is required (see 2.1).
- 1.4 and 1.5: the code blocks had been swapped between the two sections.
- 2.1 / 3.4: `filter` and `from_rgba_image` clones are avoidable, not inherent.
- 3.1: the clone is on `AnchorModeArg` (not `Copy`), not `OverlayAnchorMode`.
- 3.2: those lines are `PathBuf`s in `context.rs`, not `String`s in `convert.rs`.
- There were two sections numbered 3.3. `svg_folder_customizer.rs:180` is production code, not a test.
- Previously missing: `customizer.rs:152`, `:167`, `:180`; `solid_color.rs:189`; `convert.rs:27`, `:39`, `:74`; `cache.rs:166`; `svg_folder_customizer.rs:157` (hidden `Into<String>`); `svg.rs:203`, `:267`; all the `to_rgba8()` sites; the double-clone callers in wasm and the CLI; the `export_profile` clones.
