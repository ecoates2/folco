# Code Review — 2026-09-15

Comprehensive review of Rust and TypeScript/Svelte code for fundamental design issues.

---

## Critical Issues

### 1. Deprecated `tsify` macros causing memory leaks (Rust)

**Files:** `crates/folco-model/src/lib.rs`, `crates/folco-model/src/folder_color.rs`, `crates/folco-renderer/src/capabilities.rs`, `crates/folco-transfer/src/lib.rs`

The `tsify(into_wasm_abi, from_wasm_abi)` attributes are **deprecated** and cause memory leaks in WASM. Clippy confirms 10 warnings:

```
warning: use of deprecated constant `...`: into_wasm_abi/from_wasm_abi are
deprecated as they cause memory leaks (https://github.com/madonoharu/tsify/issues/65).
Consider using `tsify::Ts` instead.
```

**Fix:** Replace `tsify::Tsify` with `tsify::Ts` across all affected types.

---

### 2. `unimplemented!()` panic on macOS (Rust)

**File:** `crates/folco-core/src/sys/macos.rs:20`

```rust
pub fn get_folder_icon_content_bounds(width: u32, height: u32) -> RectPx {
    unimplemented!(
        "macOS folder icon content bounds not yet implemented for {}x{}",
        width, height
    )
}
```

This will **panic at runtime** if called on macOS. The Linux stub returns the full image (safe but approximate), while macOS panics.

**Fix:** Either implement proper bounds or make macOS match Linux's safe fallback:
```rust
RectPx { x: 0, y: 0, width, height }
```

---

### 3. `unreachable!()` in capabilities (Rust)

**File:** `crates/folco-renderer/src/capabilities.rs:120`

```rust
Self::RasterFolder => unreachable!(),
```

This is in `rejection()` — the code path is logically unreachable (since `supports` returns `true` for `RasterFolder`), but `unreachable!()` is still a panic. If a new `IconBaseKind` variant is added in the future and this match arm isn't updated, the panic will be silent and hard to debug.

**Fix:** Replace with a descriptive panic:
```rust
Self::RasterFolder => panic!("RasterFolder supports all layers — this branch is unreachable"),
```
Or restructure to eliminate the arm entirely.

---

## High-Priority Issues

### 4. `unwrap()` in production code paths (Rust)

**Files:**
- `crates/folco-renderer/src/layer/decal.rs:78` — `self.config().unwrap()`
- `crates/folco-renderer/src/layer/color_dot.rs:115` — `self.config().expect("...")` (good message)
- `crates/folco-renderer/src/layer/overlay.rs:192` — `self.config().unwrap()`

These are in `render_tile()` — if the layer is somehow "active" without a config (a bug in the caller), these will panic. The `expect()` messages are good, but `unwrap()` without a message is not.

**Fix:** Replace `unwrap()` with `expect()` providing context, or restructure control flow so these can't happen.

---

### 5. `expect()` for user-facing errors in Windows (Rust)

**File:** `crates/folco-core/src/sys/windows.rs:31`

```rust
let size = WindowsIconSize::from_dimension(dimension).expect("Invalid Windows icon dimension");
```

This is in `get_folder_icon_content_bounds()` which is called from `convert_icon_set()` — a user-facing path. If an unexpected dimension slips through, the user gets a panic instead of a proper error.

**Fix:** Return `Result<RectPx, ...>` or handle the error gracefully.

---

### 6. `SendableContext` unsafe impl (Rust)

**File:** `gui/src-tauri/src/state.rs:11-14`

```rust
struct SendableContext(CustomizationContext);

// SAFETY: Access is always serialized through a Mutex.
unsafe impl Send for SendableContext {}
unsafe impl Sync for SendableContext {}
```

This is **correct in principle** (Mutex provides serialization), but it's a security boundary. The safety comment is minimal.

**Fix:** Add a more detailed SAFETY comment explaining the invariant and why it holds.

---

### 7. Public struct fields on `AppInfo` (Rust)

**File:** `crates/folco-core/src/context.rs:38-44`

```rust
pub struct AppInfo {
    pub qualifier: String,
    pub organization: String,
    pub application: String,
}
```

Public fields on a public struct makes it impossible to add validation or change field types without breaking API compatibility.

**Fix:** Make fields private with getter methods, or at least document that this is a public API.

---

## Medium-Priority Issues

### 8. TODO: Linux SVG support (Rust)

**File:** `crates/folco-core/src/convert.rs:81`

```rust
// TODO: Support SVG for linux
SysIconSet { images, svg: None }
```

The conversion function always drops SVG data for Linux. Since Linux icon themes often ship SVG, this is a real gap.

---

### 9. `FolderColor` display names use match instead of data (Rust)

**File:** `crates/folco-model/src/folder_color.rs`

The `display_name()`, RGB values, and `all()` array are maintained in three separate match expressions + one array. Adding a color requires updating 4 places.

**Fix:** Store metadata as associated data on enum variants:
```rust
pub enum FolderColor {
    Red { r: u8, g: u8, b: u8, display: &'static str },
    // ...
}
```
Or use a macro to generate them consistently.

---

### 10. `parse_overlay_position` defaults to `BottomRight` for unknown input (Rust/WASM)

**File:** `crates/folco-renderer-wasm/src/canvas.rs:68-75`

```rust
fn parse_overlay_position(position: &str) -> OverlayPosition {
    match position {
        "top-left" => OverlayPosition::TopLeft,
        "top-right" => OverlayPosition::TopRight,
        "bottom-left" => OverlayPosition::BottomLeft,
        "center" => OverlayPosition::Center,
        _ => OverlayPosition::BottomRight,  // Silent fallback!
    }
}
```

A typo in the JS string silently produces `BottomRight` instead of failing. The same pattern exists in `parse_overlay_anchor_mode`.

**Fix:** Return a `Result` or `Option`, or at least log a warning.

---

### 11. TypeScript types directory is empty

**File:** `gui/src/lib/types/.gitkeep`

The types directory exists but only has a `.gitkeep`. All types are either inline or in `workflows/types.ts`.

**Fix:** Either remove the empty directory or consolidate types there.

---

### 12. Spinner rendering bug in IconPreview (Svelte)

**File:** `gui/src/lib/components/app/icon-preview/IconPreview.svelte:90`

```svelte
<!-- TODO: Revisit broken spinner rendering: renders in DOM but is invisible -->
<Spinner class="text-muted-foreground size-8" />
```

Known bug — spinner is rendered but invisible. This is a UI bug that should be investigated.

---

### 13. `ArtworkStore.setEmojiSkin` does shallow copy of nested data

**File:** `gui/src/lib/stores/artwork.svelte.ts:85-88`

```typescript
setEmojiSkin(skin: EmojiPickerSkin) {
    const current = this.#emoji;
    if (!current || current.data.skins.length <= 1) return;
    this.#emoji = { ...current, emoji: current.data.skins[skin].native, skin };
    this.#push();
}
```

The spread operator `{ ...current }` creates a shallow copy — `data` is shared between the old and new emoji objects. If `data` is mutated elsewhere, this could cause stale state.

**Fix:** Deep clone `data` or restructure to avoid sharing.

---

### 14. `#platformSizes` field is never used

**File:** `gui/src/lib/stores/renderer.svelte.ts:79`

```typescript
#platformSizes: IconSizeSpec[] | null = null;
```

This private field is declared but never assigned or read. Dead code.

**Fix:** Remove it.

---

## Low-Priority / Cleanup Issues

### 15. `FolderStrategy::unsupported_layers` uses `expect` on known-good data

**File:** `crates/folco-core/src/context.rs:242`

```rust
reason: kind
    .rejection(layer)
    .expect("unsupported_in only yields layers the base rejects"),
```

This is correct but fragile — if the contract between `unsupported_in()` and `rejection()` changes, this becomes a misleading panic.

**Fix:** Use `unreachable!()` with a description, or restructure to use the iterator directly.

---

### 16. Missing `#[must_use]` on fallible functions

Several functions in `folco-renderer` return `Result<T, RenderError>` but don't have `#[must_use]`. Ignoring a render error silently produces a broken icon.

---

### 17. `cargo clippy` shows no errors but has deprecation warnings

The clippy run completed successfully with only deprecation warnings. No actual clippy lints are firing. Consider enabling `clippy::pedantic` or `clippy::nursery` lints to catch more issues.

---

### 18. DTO comment: TypeScript generation not implemented

**File:** `gui/src-tauri/src/dto.rs:14`

```rust
// TODO: generate matching TypeScript for these via tauri-specta.
```

The TypeScript types are hand-maintained or generated separately. This is a maintenance burden — using `tauri-specta` would auto-generate types.

---

### 19. `IconCapabilities::filter()` clones unnecessarily

**File:** `crates/folco-renderer/src/capabilities.rs:178-188`

```rust
pub fn filter(&self, profile: &CustomizationProfile) -> CustomizationProfile {
    CustomizationProfile {
        solid_color: self.solid_color.then(|| profile.solid_color.clone()).flatten(),
        // ...
    }
}
```

This clones the entire profile contents even though it's just filtering which fields to keep. For `SolidColorConfig` etc., this is a shallow clone but still unnecessary allocation.

**Fix:** Not urgent, but consider accepting `&Self` and constructing without clone where possible.

---

## Summary Table

| # | Severity | Issue | File(s) | Effort |
|---|----------|-------|---------|--------|
| 1 | Critical | Deprecated tsify macros (memory leaks) | folco-model, folco-renderer, folco-transfer | Low |
| 2 | Critical | `unimplemented!()` panic on macOS | folco-core/sys/macos.rs | Medium |
| 3 | High | `unreachable!()` in capabilities | folco-renderer/capabilities.rs | Low |
| 4 | High | `unwrap()` in render_tile | folco-renderer/layer/*.rs | Low |
| 5 | High | `expect()` for user-facing errors | folco-core/sys/windows.rs | Low |
| 6 | Medium | `SendableContext` unsafe impl | gui/src-tauri/state.rs | Low |
| 7 | Medium | Public struct fields | folco-core/context.rs | Low |
| 8 | Medium | Silent default in parse functions | folco-renderer-wasm/canvas.rs | Low |
| 9 | Medium | Dead code `#platformSizes` | gui/renderer.svelte.ts | Low |
| 10 | Medium | Spinner invisible bug | gui/icon-preview/IconPreview.svelte | Medium |
| 11 | Low | Shallow copy in emoji skin | gui/artwork.svelte.ts | Low |
| 12 | Low | TODO: Linux SVG support | folco-core/convert.rs | High |
| 13 | Low | `FolderColor` maintenance burden | folco-model/folder_color.rs | Medium |
| 14 | Low | Empty types directory | gui/src/lib/types/ | Low |
| 15 | Low | TypeScript DTO generation TODO | gui/src-tauri/dto.rs | Medium |
| 16 | Low | `expect` on known-good data | folco-core/context.rs | Low |
| 17 | Low | Missing `#[must_use]` on fallible functions | folco-renderer | Low |
| 18 | Low | Clippy lints not enabled | workspace config | Low |
| 19 | Low | Unnecessary clones in filter() | folco-renderer/capabilities.rs | Low |

---

## Top 3 to Fix Now

1. **Replace deprecated `tsify::Tsify` with `tsify::Ts`** — memory leak in production WASM usage
2. **Fix macOS `unimplemented!()`** — currently crashes on macOS
3. **Replace `unwrap()` in render_tile** — silent panics during rendering
