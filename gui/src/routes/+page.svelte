<script lang="ts">
  import { onMount } from 'svelte';
  import { renderer } from '$lib/stores/renderer.svelte';
  import { artwork } from '$lib/stores/artwork.svelte';
  import { workflow } from '$lib/stores/workflow.svelte';
  import { WORKFLOWS } from '$lib/workflows/definitions';
  import type { WorkflowId } from '$lib/workflows/types';

  import { ColorPicker } from '$lib/components/app/color-picker';
  import { CustomizationOption } from '$lib/components/app/customization-option';
  import { DirectoryPicker } from '$lib/components/app/directory-picker';
  import { IconPicker } from '$lib/components/app/icon-picker';
  import { IconPreview } from '$lib/components/app/icon-preview';
  import { SourceImagePicker } from '$lib/components/app/source-image-picker';
  import { ThemeSelector } from '$lib/components/app/theme-selector';

  import * as RadioGroup from '$lib/components/ui/radio-group';

  import { open } from '@tauri-apps/plugin-dialog';

  import * as EmojiPicker from '$lib/components/ui/emoji-picker';

  onMount(() => workflow.activate('folder'));

  const WORKFLOW_IDS = Object.keys(WORKFLOWS) as WorkflowId[];

  let solidColorEnabled = $state(false);
  let colorDotEnabled = $state(false);

  function handleIconSelect(e: Event) {
    const detail = (e as CustomEvent).detail;
    if (detail?.svg) artwork.selectIcon(detail.svg);
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
    <h1 class="text-2xl font-bold text-foreground">{workflow.spec.title}</h1>
    <ThemeSelector />
  </div>

  <RadioGroup.Root
    value={workflow.id}
    onValueChange={(v) => workflow.activate(v as WorkflowId)}
    class="mb-6 flex flex-row gap-4"
  >
    {#each WORKFLOW_IDS as id (id)}
      <div class="flex items-center gap-2">
        <RadioGroup.Item value={id} id="workflow-{id}" />
        <label for="workflow-{id}" class="cursor-pointer text-sm font-medium text-foreground">
          {WORKFLOWS[id].title}
        </label>
      </div>
    {/each}
  </RadioGroup.Root>

  <div class="mb-6 flex flex-col gap-3">
    {#each workflow.sections as section (section.id)}
      <CustomizationOption
        label={section.label}
        disabled={!section.available}
        disabledReason={section.reason}
        enabled={section.id === 'source-image' ||
          (section.id === 'solid-color' && solidColorEnabled) ||
          (section.id === 'color-dot' && colorDotEnabled) ||
          (section.id === 'emoji' && artwork.kind === 'emoji') ||
          (section.id === 'icon' && artwork.kind === 'icon')}
        onToggle={(on) => {
          if (section.id === 'solid-color') { solidColorEnabled = on; renderer.setSolidColorEnabled(on); }
          else if (section.id === 'color-dot') { colorDotEnabled = on; renderer.setColorDotEnabled(on); }
          else if (section.id === 'emoji') artwork.setKind(on ? 'emoji' : 'none');
          else if (section.id === 'icon') artwork.setKind(on ? 'icon' : 'none');
        }}
      >
        {#if section.id === 'source-image'}
          <SourceImagePicker />
        {:else if section.id === 'solid-color'}
          <ColorPicker
            onchange={(color) => renderer.setSolidColor(color.r, color.g, color.b)}
          />
        {:else if section.id === 'color-dot'}
          <ColorPicker
            onchange={(color) => renderer.setColorDot(color.r, color.g, color.b)}
          />
        {:else if section.id === 'emoji'}
          <RadioGroup.Root
            value={artwork.anchorMode}
            onValueChange={(v) => { if (v === 'inset' || v === 'centered') artwork.setAnchorMode(v); }}
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
          <EmojiPicker.Root
            onSelect={(emoji) => artwork.selectEmoji(emoji)}
            onSkinChange={(skin) => artwork.setEmojiSkin(skin)}
          >
            <EmojiPicker.Viewport>
              <EmojiPicker.Search />
              <EmojiPicker.List />
              <EmojiPicker.Footer>
                <EmojiPicker.SkinToneSelector />
              </EmojiPicker.Footer>
            </EmojiPicker.Viewport>
          </EmojiPicker.Root>
        {:else if section.id === 'icon'}
          <RadioGroup.Root
            value={artwork.effectivePlacement}
            onValueChange={(v) => { if (v === 'decal' || v === 'overlay') artwork.setPlacement(v); }}
            class="mb-3 flex flex-row gap-4"
          >
            <div class="flex items-center gap-2">
              <RadioGroup.Item value="decal" id="icon-mode-decal" disabled={!renderer.supportsDecal} />
              <label
                for="icon-mode-decal"
                class="cursor-pointer text-sm font-medium text-foreground {renderer.supportsDecal
                  ? ''
                  : 'cursor-not-allowed opacity-50'}"
                title={renderer.rejections.decal}
              >Decal</label>
            </div>
            <div class="flex items-center gap-2">
              <RadioGroup.Item value="overlay" id="icon-mode-overlay" />
              <label for="icon-mode-overlay" class="cursor-pointer text-sm font-medium text-foreground">Overlay</label>
            </div>
          </RadioGroup.Root>
          {#if artwork.effectivePlacement === 'overlay'}
            <RadioGroup.Root
              value={artwork.anchorMode}
              onValueChange={(v) => { if (v === 'inset' || v === 'centered') artwork.setAnchorMode(v); }}
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
        {/if}
      </CustomizationOption>
    {/each}
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
