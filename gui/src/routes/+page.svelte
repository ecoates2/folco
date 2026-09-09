<script lang="ts">
  import { onMount } from 'svelte';
  import { renderer } from '$lib/stores/renderer.svelte';

  import { ColorPicker } from '$lib/components/app/color-picker';
  import { CustomizationOption } from '$lib/components/app/customization-option';
  import { DirectoryPicker } from '$lib/components/app/directory-picker';
  import { IconPicker } from '$lib/components/app/icon-picker';
  import { IconPreview } from '$lib/components/app/icon-preview';
  import { ThemeSelector } from '$lib/components/app/theme-selector';

  import * as RadioGroup from '$lib/components/ui/radio-group';

  import { open } from '@tauri-apps/plugin-dialog';

  import * as EmojiPicker from '$lib/components/ui/emoji-picker';
  import type { SelectedEmoji, EmojiPickerSkin } from '$lib/components/ui/emoji-picker/types';

  onMount(() => renderer.init());

  let solidColorEnabled = $state(false);
  let colorDotEnabled = $state(false);
  let emojiEnabled = $state(false);
  let iconEnabled = $state(false);

  /** Whether the selected icon SVG is applied as a decal or overlay. */
  let iconMode = $state<'decal' | 'overlay'>('decal');

  // Vector folder icons have no decal layer; keep the UI on a mode that works.
  $effect(() => {
    if (!renderer.supportsDecal && iconMode === 'decal') iconMode = 'overlay';
  });

  /** How overlays attach to their chosen anchor point. */
  let overlayAnchorMode = $state<'inset' | 'centered'>('inset');

  /** Last selected emoji — used to re-emit the overlay when the skin tone changes. */
  let lastEmoji = $state<SelectedEmoji | null>(null);

  /** Last selected icon SVG markup. */
  let lastIconSvg = $state<string | null>(null);

  const OVERLAY_POSITION = 'bottom-right';
  const OVERLAY_SCALE = 0.5;
  const DECAL_SCALE = 0.7;

  /**
   * Pushes the complete decal + overlay state to the renderer.
   *
   * Emoji and icon overlays share a single overlay layer, so the state is always
   * rewritten in full: only toggling the layer that gained focus would leave the
   * previous source configured, and it would keep rendering.
   */
  function syncImageLayers() {
    const emoji = emojiEnabled ? lastEmoji : null;
    const icon = iconEnabled ? lastIconSvg : null;

    renderer.setDecal(iconMode === 'decal' ? icon : null, DECAL_SCALE);

    if (emoji) {
      renderer.setOverlayEmoji(emoji.emoji, OVERLAY_POSITION, overlayAnchorMode, OVERLAY_SCALE);
    } else {
      const overlay = iconMode === 'overlay' ? icon : null;
      renderer.setOverlay(overlay, OVERLAY_POSITION, overlayAnchorMode, OVERLAY_SCALE);
    }
  }

  function setOverlayAnchorMode(mode: 'inset' | 'centered') {
    overlayAnchorMode = mode;
    syncImageLayers();
  }

  // ── Emoji handlers ───────────────────────────────────────────────

  function enableEmoji(on: boolean) {
    // Emoji and icon are mutually exclusive
    if (on && iconEnabled) iconEnabled = false;
    emojiEnabled = on;
    syncImageLayers();
  }

  function handleEmojiSelect(emoji: SelectedEmoji) {
    lastEmoji = emoji;
    syncImageLayers();
  }

  function handleSkinChange(skin: EmojiPickerSkin) {
    if (!lastEmoji || lastEmoji.data.skins.length <= 1) return;
    lastEmoji = { ...lastEmoji, emoji: lastEmoji.data.skins[skin].native, skin };
    syncImageLayers();
  }

  // ── Icon handlers ────────────────────────────────────────────────

  function enableIcon(on: boolean) {
    // Emoji and icon are mutually exclusive
    if (on && emojiEnabled) emojiEnabled = false;
    iconEnabled = on;
    syncImageLayers();
  }

  function handleIconSelect(e: Event) {
    const detail = (e as CustomEvent).detail;
    if (!detail?.svg) return;
    lastIconSvg = detail.svg;
    syncImageLayers();
  }

  /** Switches the icon between decal and overlay mode, moving the SVG data. */
  function setIconMode(mode: 'decal' | 'overlay') {
    if (mode === iconMode) return;
    iconMode = mode;
    syncImageLayers();
  }

  // ── Directory handlers ───────────────────────────────────────────

  let directories = $state<string[]>([]);
  let selectedIndex = $state<number | null>(null);

  async function handleAdd() {
    const selected = await open({
      directory: true,
      multiple: true,
      title: 'Select Folder(s)'
    });

    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected];
      directories = [...directories, ...paths.filter(p => !directories.includes(p))];
    }
  }

  function handleRemove(index: number) {
    directories = directories.filter((_, i) => i !== index);
    selectedIndex = null;
  }

  function handleClearAll() {
    directories = [];
    selectedIndex = null;
  }

  function handleDropPaths(paths: string[]) {
    const newPaths = paths.filter(p => !directories.includes(p));
    directories = [...directories, ...newPaths];
  }
</script>

<main class="container mx-auto max-w-2xl p-6">
  <div class="mb-6 flex items-center justify-between">
    <h1 class="text-2xl font-bold text-foreground">Folder Customization</h1>
    <ThemeSelector />
  </div>

  <div class="mb-6 flex flex-col gap-3">
    <CustomizationOption
      label="Solid Color"
      bind:enabled={solidColorEnabled}
      disabled={!renderer.supportsSolidColor}
      disabledReason="This system's folder icon is a scalable SVG, which can't be recolored. Use a color dot instead."
      onToggle={(on) => renderer.setSolidColorEnabled(on)}
    >
      <ColorPicker
        onchange={(color) => renderer.setSolidColor(color.r, color.g, color.b)}
      />
    </CustomizationOption>

    <CustomizationOption
      label="Color Dot"
      bind:enabled={colorDotEnabled}
      onToggle={(on) => renderer.setColorDotEnabled(on)}
    >
      <ColorPicker
        onchange={(color) => renderer.setColorDot(color.r, color.g, color.b)}
      />
    </CustomizationOption>

    <CustomizationOption
      label="Emoji Overlay"
      bind:enabled={emojiEnabled}
      onToggle={enableEmoji}
    >
      <RadioGroup.Root
        value={overlayAnchorMode}
        onValueChange={(v) => { if (v === 'inset' || v === 'centered') setOverlayAnchorMode(v); }}
        class="mb-3 flex flex-row gap-4"
      >
        <div class="flex items-center gap-2">
          <RadioGroup.Item value="inset" id="emoji-anchor-inset" />
          <label for="emoji-anchor-inset" class="cursor-pointer text-sm font-medium text-foreground">Inset</label>
        </div>
        <div class="flex items-center gap-2">
          <RadioGroup.Item value="centered" id="emoji-anchor-centered" />
          <label for="emoji-anchor-centered" class="cursor-pointer text-sm font-medium text-foreground">Centered</label>
        </div>
      </RadioGroup.Root>
      <EmojiPicker.Root onSelect={handleEmojiSelect} onSkinChange={handleSkinChange}>
        <EmojiPicker.Viewport>
          <EmojiPicker.Search />
          <EmojiPicker.List />
          <EmojiPicker.Footer>
            <EmojiPicker.SkinToneSelector />
          </EmojiPicker.Footer>
        </EmojiPicker.Viewport>
      </EmojiPicker.Root>
    </CustomizationOption>

    <CustomizationOption
      label="Icon"
      bind:enabled={iconEnabled}
      onToggle={enableIcon}
    >
      <RadioGroup.Root
        value={iconMode}
        onValueChange={(v) => { if (v === 'decal' || v === 'overlay') setIconMode(v); }}
        class="mb-3 flex flex-row gap-4"
      >
        <div class="flex items-center gap-2">
          <RadioGroup.Item value="decal" id="icon-mode-decal" disabled={!renderer.supportsDecal} />
          <label
            for="icon-mode-decal"
            class="cursor-pointer text-sm font-medium text-foreground {renderer.supportsDecal
              ? ''
              : 'cursor-not-allowed opacity-50'}"
          >Decal</label>
        </div>
        <div class="flex items-center gap-2">
          <RadioGroup.Item value="overlay" id="icon-mode-overlay" />
          <label for="icon-mode-overlay" class="cursor-pointer text-sm font-medium text-foreground">Overlay</label>
        </div>
      </RadioGroup.Root>
      {#if iconMode === 'overlay'}
        <RadioGroup.Root
          value={overlayAnchorMode}
          onValueChange={(v) => { if (v === 'inset' || v === 'centered') setOverlayAnchorMode(v); }}
          class="mb-3 flex flex-row gap-4"
        >
          <div class="flex items-center gap-2">
            <RadioGroup.Item value="inset" id="icon-anchor-inset" />
            <label for="icon-anchor-inset" class="cursor-pointer text-sm font-medium text-foreground">Inset</label>
          </div>
          <div class="flex items-center gap-2">
            <RadioGroup.Item value="centered" id="icon-anchor-centered" />
            <label for="icon-anchor-centered" class="cursor-pointer text-sm font-medium text-foreground">Centered</label>
          </div>
        </RadioGroup.Root>
      {/if}
      <!-- Search function currently broken... -->
      <IconPicker
        collection="mdi"
        hideSearch
        pageSize={30}
        oniconselected={handleIconSelect}>
      </IconPicker>
    </CustomizationOption>
  </div>

  <IconPreview class="mb-6" />

  <DirectoryPicker
    bind:directories
    bind:selectedIndex
    onAdd={handleAdd}
    onRemove={handleRemove}
    onClearAll={handleClearAll}
    onDropPaths={handleDropPaths}
    class="max-w-md"
  />

  <!-- Debug output -->
  <div class="mt-6 rounded-md border border-border bg-muted/50 p-4">
    <p class="text-sm text-muted-foreground">
      Selected directories: {directories.length}
    </p>
    <p class="text-sm text-muted-foreground">
      Selected index: {selectedIndex ?? 'none'}
    </p>
  </div>
</main>

<style lang="postcss">
  @reference "tailwindcss";
  :global(html) {
    background-color: theme(--color-gray-100);
  }
</style>
