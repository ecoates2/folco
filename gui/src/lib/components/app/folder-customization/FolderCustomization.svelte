<script lang="ts">
	import { onMount } from 'svelte';
	import { renderer } from '$lib/stores/renderer.svelte';
	import { artwork } from '$lib/stores/artwork.svelte';
	import { solidColor } from '$lib/stores/solid-color.svelte';
	import { features } from '$lib/stores/features.svelte';
	import { workflow } from '$lib/stores/workflow.svelte';
	import { directories } from '$lib/stores/directories.svelte';
	import { folderOperations, type FolderOperation } from '$lib/stores/folder-operations.svelte';
	import { WORKFLOWS } from '$lib/workflows/definitions';
	import type { WorkflowId } from '$lib/workflows/types';
	import { cn } from '$lib/utils';

	import { ColorPicker } from '$lib/components/app/color-picker';
	import { CustomizationOption } from '$lib/components/app/customization-option';
	import { DirectoryPicker } from '$lib/components/app/directory-picker';
	import { IconPicker } from '$lib/components/app/icon-picker';
	import { IconPreview } from '$lib/components/app/icon-preview';
	import { SourceImagePicker } from '$lib/components/app/source-image-picker';
	import { ThemeSelector } from '$lib/components/app/theme-selector';

	import * as RadioGroup from '$lib/components/ui/radio-group';
	import * as EmojiPicker from '$lib/components/ui/emoji-picker';
	import { Button } from '$lib/components/ui/button';
	// Not ui/spinner: its null-forwarding renders an invisible icon.
	import Loader2Icon from '@lucide/svelte/icons/loader-2';

	interface Props {
		class?: string;
	}

	let { class: className }: Props = $props();

	onMount(() => workflow.activate('folder'));

	const WORKFLOW_IDS = Object.keys(WORKFLOWS) as WorkflowId[];

	const OPERATION_LABELS: Record<FolderOperation, { idle: string; running: string; done: string }> = {
		apply: { idle: 'Apply', running: 'Applying', done: 'applied' },
		reset: { idle: 'Reset', running: 'Resetting', done: 'reset' }
	};
</script>

{#snippet operationButton(
	operation: FolderOperation,
	blockedReason: string | null,
	variant: 'default' | 'outline',
	onclick: () => void
)}
	<!-- Disabled buttons swallow pointer events, so the tooltip lives on a wrapper. -->
	<span title={blockedReason ?? undefined}>
		<Button {variant} {onclick} disabled={blockedReason !== null}>
			{#if folderOperations.status === 'running' && folderOperations.operation === operation}
				<Loader2Icon class="size-4 animate-spin" />
				{OPERATION_LABELS[operation].running} {folderOperations.processed}/{folderOperations.total}
			{:else}
				{OPERATION_LABELS[operation].idle}
			{/if}
		</Button>
	</span>
{/snippet}

<div class={cn(className)}>
	<div class="mb-6 flex items-center justify-between">
		<h1 class="text-2xl font-bold text-foreground">Folder Customization</h1>
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
				toggleable={section.id !== 'source-image'}
				enabled={(section.id === 'solid-color' && solidColor.enabled) ||
					(section.id === 'color-dot' && artwork.kind === 'colorDot') ||
					(section.id === 'emoji' && artwork.kind === 'emoji') ||
					(section.id === 'icon' && artwork.kind === 'icon')}
				onToggle={(on) => {
					if (section.id === 'solid-color') solidColor.setEnabled(on);
					else if (section.id === 'color-dot') artwork.setKind(on ? 'colorDot' : 'none');
					else if (section.id === 'emoji') artwork.setKind(on ? 'emoji' : 'none');
					else if (section.id === 'icon') artwork.setKind(on ? 'icon' : 'none');
				}}
			>
				{#if section.id === 'source-image'}
					<SourceImagePicker />
				{:else if section.id === 'solid-color'}
					<ColorPicker
						value={solidColor.selectedId}
						onchange={(color) => solidColor.select(color)}
					/>
				{:else if section.id === 'color-dot'}
					{#if features.experimental}
						<RadioGroup.Root
							value={artwork.anchorMode}
							onValueChange={(v) => { if (v === 'inset' || v === 'centered') artwork.setAnchorMode(v); }}
							class="mb-3 flex flex-row gap-4"
						>
							<div class="flex items-center gap-2">
								<RadioGroup.Item value="inset" id="color-dot-anchor-inset" />
								<label for="color-dot-anchor-inset" class="cursor-pointer text-sm font-medium text-foreground">Inset</label>
							</div>
							<div class="flex items-center gap-2">
								<RadioGroup.Item value="centered" id="color-dot-anchor-centered" />
								<label for="color-dot-anchor-centered" class="cursor-pointer text-sm font-medium text-foreground">Centered</label>
							</div>
						</RadioGroup.Root>
					{/if}
					<ColorPicker
						value={artwork.colorDotId}
						onchange={(color) => artwork.selectColorDot(color)}
					/>
				{:else if section.id === 'emoji'}
					{#if features.experimental}
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
					{/if}
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
					{#if features.experimental && artwork.effectivePlacement === 'overlay'}
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
						onselect={(svg) => artwork.selectIcon(svg)}>
					</IconPicker>
				{/if}
			</CustomizationOption>
		{/each}
	</div>

	<IconPreview class="mb-6" />

	<DirectoryPicker
		bind:directories={directories.paths}
		bind:selectedIndex={directories.selectedIndex}
		onAdd={() => directories.browse()}
		onRemove={(i) => directories.remove(i)}
		onClearAll={() => directories.clear()}
		onDropPaths={(paths) => directories.add(paths)}
		class="max-w-md"
	/>

	<div class="mt-6 flex items-center gap-3">
		{@render operationButton('apply', folderOperations.applyBlockedReason, 'default', () =>
			folderOperations.apply()
		)}
		{@render operationButton('reset', folderOperations.resetBlockedReason, 'outline', () =>
			folderOperations.reset()
		)}

		{#if folderOperations.status === 'done' && !folderOperations.error}
			<span class="text-sm text-muted-foreground">
				{folderOperations.succeeded}
				{OPERATION_LABELS[folderOperations.operation].done}{folderOperations.failures.length
					? `, ${folderOperations.failures.length} failed`
					: ''}
			</span>
		{/if}
	</div>
</div>
