# Renderer Refactor Plan

## Problem Statement

Two architectural issues make the renderer hard to reason about:

1. **`CustomizationProfile` is a union type shared by all workflows** — every customizer accepts the same struct with `solidColor`, `colorDot`, `decal`, `overlay`, even though folder and custom workflows use different subsets. The dead fields are filtered at runtime via `capabilities().filter()`.

2. **`enabled` is a UI concept leaking into the library** — `Layer<C>` owns `config: Option<C>` AND `enabled: bool`, but the profile has no `enabled` field. `Some(config)` = on, `None` = off in profiles. The frontend has two separate calls per layer: `setColorDot(r,g,b)` and `setColorDotEnabled(bool)`. Export/import is lossless for config but ignores the toggle.

## Goals

- Compile-time safety: no dead fields can reach a customizer that can't realize them
- One concept per layer: presence/absence of config = on/off, no separate toggle
- Lossless export/import: what you see is what you get
- Minimal breaking changes to CLI and JSON wire format

## Steps

### Step 1: Remove `enabled` from Layer

**Files changed:** folco-renderer, folco-renderer-wasm, gui

**Rationale:** The `enabled` flag is the source of the config/active_config confusion. `Some(config)` should mean "active", `None` should mean "inactive".

**Changes:**
- `Layer<C>`: remove `enabled: bool` field
- `Layer<C>`: remove `is_enabled()`, `set_enabled()` methods
- `Layer<C>`: `is_active()` → `is_some()` (check `config.is_some()`)
- `Layer<C>`: `active_config()` → `config()` (return `self.config` directly)
- `Layer<C>`: `has_config()` → `is_some()` (check `config.is_some()`)
- `Layer<C>`: `apply_config()` → `apply()` (sets config, no enabled toggle)
- `Layer<C>`: `set_config()` → `set()` (sets config, clears cache)
- `SvgLayer<C>`: same renames as Layer
- `CompositeLayer`: unchanged (no config)

**folco-renderer-wasm changes:**
- Remove `setSolidColorEnabled()`, `setColorDotEnabled()`, `setDecalEnabled()`, `setOverlayEnabled()` from CanvasRenderer
- Each layer method (`setSolidColor`, `setColorDot`, etc.) now takes the full config OR null to clear

**GUI changes:**
- `renderer.svelte.ts`: remove `setSolidColorEnabled()`, `setColorDotEnabled()`, `setDecalEnabled()`, `setOverlayEnabled()`
- `+page.svelte`: remove `solidColorEnabled` and `colorDotEnabled` state variables
- `+page.svelte`: change toggle handler — instead of `setSolidColorEnabled(on)`, call `renderer.setSolidColor(on ? currentColor : null)`
- `CustomizationOption.svelte`: the `enabled` prop becomes read-only (driven by config presence), not a toggle

### Step 2: Split CustomizationProfile into workflow-specific types

**Files changed:** folco-renderer, folco-core, folco-cli, folco-renderer-wasm, gui

**Changes:**

**folco-renderer:**
- Keep `CustomizationProfile` as the **wire format** (JSON serializable, union type) — this stays for backward compat and CLI args
- Add `FolderProfile` struct with `solidColor`, `colorDot`, `decal`, `overlay`
- Add `CustomProfile` struct with `colorDot`, `overlay`
- Add `From<&CustomizationProfile> for FolderProfile` and `From<&CustomizationProfile> for CustomProfile` conversions
- `FolderIconCustomizer::apply_profile()` accepts `&FolderProfile`
- `CustomIconCustomizer::apply_profile()` accepts `&CustomProfile`
- `SvgFolderIconCustomizer::apply_profile()` accepts `&FolderProfile` (SVG folder uses same fields minus decal/solidColor, but those are filtered at construction)
- `export_profile()` returns `CustomizationProfile` (wire format)

**folco-core:**
- `CustomizationContext::apply_profile()` accepts `&FolderProfile`
- `CustomizationContext::customize_folders()` accepts `&FolderProfile`
- `CustomizationContext::create_custom_icon_customizer()` returns a customizer that accepts `&CustomProfile`
- Add `FolderProfile` and `CustomProfile` types (re-export from folco-renderer)

**folco-cli:**
- `--folder-customization-profile` parses to `CustomizationProfile` → converts to `FolderProfile`
- `--custom-icon-profile` parses to `CustomizationProfile` → converts to `CustomProfile`
- No user-visible change

**folco-renderer-wasm:**
- Keep `exportProfileJson()` / `importProfileJson()` using `CustomizationProfile` (wire format)
- Add `importFolderProfileJson()` / `importCustomProfileJson()` that parse the specific type

**GUI:**
- `renderer.svelte.ts`: `importProfileJson()` stays as-is (wire format)
- When importing, the frontend constructs the right profile type for the workflow

### Step 3: Rename Layer methods for clarity

**Files changed:** folco-renderer, folco-renderer-wasm, gui (tests)

**Rationale:** After Step 1, the naming should be unambiguous:

| Old name | New name | Reason |
|----------|----------|--------|
| `config()` | `config()` | stays — returns the optional config |
| `active_config()` | `config()` | merged with `config()` in Step 1 |
| `has_config()` | `is_some()` | clearer boolean |
| `set_config()` | `set()` | sets config, no enabled concept |
| `apply_config()` | `apply()` | profile is source of truth, sets + enables |
| `is_active()` | `is_some()` | same as `is_some()` after Step 1 |
| `is_enabled()` | _(removed)_ | no longer exists |

**Actually, let's reconsider naming:**

| Old name | New name | Reason |
|----------|----------|--------|
| `config()` | `config()` | unchanged |
| `active_config()` | _(merged)_ | removed after Step 1 |
| `has_config()` | `is_configured()` | clearer than `is_some()` |
| `set_config()` | `set_config()` | unchanged (or `set()`) |
| `apply_config()` | `apply()` | profile-driven, no enabled |
| `is_active()` | `is_active()` | unchanged (config.is_some()) |
| `is_enabled()` | _(removed)_ | gone |

### Step 4: Update all layer implementations

**Files changed:** folco-renderer/src/layer/*.rs, folco-renderer/src/svg_layer/*.rs

- Update all `self.config()` → `self.config()` (no change, just verify)
- Update all `self.active_config()` → `self.config()` (merged)
- Update all `self.has_config()` → `self.is_configured()`
- Update all `self.set_config()` → `self.set_config()` (no change)
- Update all `self.apply_config()` → `self.apply()`
- Update all `self.is_active()` → `self.is_active()` (no change)
- Update all `self.is_enabled()` / `self.set_enabled()` → removed
- Update all callers in tests

### Step 5: Update doc examples and tests

**Files changed:** All files with doc examples and test code

- Update all doc examples (`lib.rs`, `customizer.rs`, `folder_customizer.rs`, `custom_customizer.rs`, `svg_folder_customizer.rs`)
- Update all test assertions

### Step 6: Verify build and tests

- `cargo check --all-features`
- `cargo test --all-features`
- `cargo test --all-features --release`
- Check WASM build: `cargo build --target wasm32-unknown-unknown -p folco-renderer-wasm`
- Check GUI: `cd gui && npm run build`
