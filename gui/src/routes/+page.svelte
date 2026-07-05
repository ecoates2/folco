<script lang="ts">
  import { onMount } from 'svelte';
  import { renderer } from '$lib/stores/renderer.svelte';

  import { ColorPicker } from '$lib/components/app/color-picker';
  import { CustomizationOption } from '$lib/components/app/customization-option';
  import { DirectoryPicker } from '$lib/components/app/directory-picker';
  import { IconPreview } from '$lib/components/app/icon-preview';

  import * as RadioGroup from '$lib/components/ui/radio-group';

  import 'iconify-picker';

  import { open } from '@tauri-apps/plugin-dialog';

  import * as EmojiPicker from '$lib/components/ui/emoji-picker';
  import type { SelectedEmoji, EmojiPickerSkin } from '$lib/components/ui/emoji-picker/types';

  onMount(() => renderer.init());

  let colorEnabled = $state(false);
  let emojiEnabled = $state(false);
  let iconEnabled = $state(false);

  /** Whether the selected icon SVG is applied as a decal or overlay. */
  let iconMode = $state<'decal' | 'overlay'>('decal');

  /** How overlays attach to their chosen anchor point. */
  let overlayAnchorMode = $state<'inset' | 'centered'>('inset');

  /** Last selected emoji — used to re-emit the overlay when the skin tone changes. */
  let lastEmoji = $state<SelectedEmoji | null>(null);

  /** Last selected icon SVG markup. */
  let lastIconSvg = $state<string | null>(null);

  // ── Emoji handlers ───────────────────────────────────────────────

  function enableEmoji(on: boolean) {
    if (on && iconEnabled) {
      // Disable icon — they are mutually exclusive
      iconEnabled = false;
      clearIcon();
    }
    renderer.setOverlayEnabled(on);
  }

  function handleEmojiSelect(emoji: SelectedEmoji) {
    lastEmoji = emoji;
    renderer.setOverlayEmoji(emoji.emoji, 'bottom-right', overlayAnchorMode, 0.5);
  }

  function handleSkinChange(skin: EmojiPickerSkin) {
    if (!lastEmoji || lastEmoji.data.skins.length <= 1) return;
    const native = lastEmoji.data.skins[skin].native;
    lastEmoji = { ...lastEmoji, emoji: native, skin };
    renderer.setOverlayEmoji(native, 'bottom-right', overlayAnchorMode, 0.5);
  }

  function setOverlayAnchorMode(mode: 'inset' | 'centered') {
    overlayAnchorMode = mode;

    if (emojiEnabled && lastEmoji) {
      renderer.setOverlayEmoji(lastEmoji.emoji, 'bottom-right', overlayAnchorMode, 0.5);
    }

    if (iconEnabled && iconMode === 'overlay' && lastIconSvg) {
      renderer.setOverlay(lastIconSvg, 'bottom-right', overlayAnchorMode, 0.5);
    }
  }

  // ── Icon handlers ────────────────────────────────────────────────

  function enableIcon(on: boolean) {
    if (on && emojiEnabled) {
      // Disable emoji — they are mutually exclusive
      emojiEnabled = false;
      clearEmoji();
    }
    // Enable whichever layer the icon is targeting
    if (iconMode === 'decal') {
      renderer.setDecalEnabled(on);
    } else {
      renderer.setOverlayEnabled(on);
    }
  }

  function handleIconSelect(e: Event) {
    const detail = (e as CustomEvent).detail;
    if (!detail?.svg) return;
    lastIconSvg = detail.svg;
    applyIconSvg(detail.svg);
  }

  /** Applies the current icon SVG to the layer indicated by `iconMode`. */
  function applyIconSvg(svg: string) {
    if (iconMode === 'decal') {
      renderer.setDecal(svg, 0.7);
    } else {
      renderer.setOverlay(svg, 'bottom-right', overlayAnchorMode, 0.5);
    }
  }

  /** Switches the icon between decal and overlay mode, moving the SVG data. */
  function setIconMode(mode: 'decal' | 'overlay') {
    if (mode === iconMode) return;

    // Disable the old layer
    if (iconMode === 'decal') {
      renderer.setDecalEnabled(false);
    } else {
      renderer.setOverlayEnabled(false);
    }

    iconMode = mode;

    // Re-apply SVG to the new layer and enable it
    if (lastIconSvg) {
      applyIconSvg(lastIconSvg);
    }
    if (iconMode === 'decal') {
      renderer.setDecalEnabled(true);
    } else {
      renderer.setOverlayEnabled(true);
    }
  }

  // ── Cleanup helpers ──────────────────────────────────────────────

  function clearEmoji() {
    renderer.setOverlayEnabled(false);
    lastEmoji = null;
  }

  function clearIcon() {
    renderer.setDecalEnabled(false);
    renderer.setOverlayEnabled(false);
    lastIconSvg = null;
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
  <h1 class="mb-6 text-2xl font-bold text-foreground">Folder Customization</h1>

  <div class="mb-6 flex flex-col gap-3">
    <CustomizationOption
      label="Color"
      bind:enabled={colorEnabled}
      onToggle={(on) => renderer.setFolderColorTargetEnabled(on)}
    >
      <ColorPicker
        onchange={(color) => renderer.setFolderColorTarget(color.r, color.g, color.b)}
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
          <RadioGroup.Item value="decal" id="icon-mode-decal" />
          <label for="icon-mode-decal" class="cursor-pointer text-sm font-medium text-foreground">Decal</label>
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
      <iconify-picker
        collection="mdi"
        hide-search
        page-size="30"
        onicon-selected={handleIconSelect}>
      </iconify-picker>
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
