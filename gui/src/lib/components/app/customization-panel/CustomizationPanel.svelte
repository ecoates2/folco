<script lang="ts">
	import { tick, untrack, type Snippet } from 'svelte';
	import { artwork, type ArtworkKind } from '$lib/stores/artwork.svelte';
	import { features } from '$lib/stores/features.svelte';
	import { renderer } from '$lib/stores/renderer.svelte';
	import { solidColor } from '$lib/stores/solid-color.svelte';
	import { workflow } from '$lib/stores/workflow.svelte';
	import type { SectionId } from '$lib/workflows/types';
	import { cn } from '$lib/utils';

	import { ColorPicker } from '$lib/components/app/color-picker';
	import { IconPicker } from '$lib/components/app/icon-picker';
	import { PickerPanel } from '$lib/components/app/picker-panel';
	import { SourceImagePicker } from '$lib/components/app/source-image-picker';

	import { Button } from '$lib/components/ui/button';
	import * as EmojiPicker from '$lib/components/ui/emoji-picker';
	import * as RadioGroup from '$lib/components/ui/radio-group';
	import * as Tabs from '$lib/components/ui/tabs';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

	interface Props {
		class?: string;
	}

	let { class: className }: Props = $props();

	interface Choice {
		value: string;
		label: string;
		disabled?: boolean;
		title?: string;
	}

	const OVERLAY_TABS: { kind: Exclude<ArtworkKind, 'none'>; section: SectionId; label: string }[] = [
		{ kind: 'colorDot', section: 'color-dot', label: 'Dot' },
		{ kind: 'emoji', section: 'emoji', label: 'Emoji' },
		{ kind: 'icon', section: 'icon', label: 'Icon' }
	];

	const ANCHOR_CHOICES: Choice[] = [
		{ value: 'inset', label: 'Inset' },
		{ value: 'centered', label: 'Centered' }
	];

	const HEADING = 'text-xs font-semibold uppercase tracking-wide text-muted-foreground';
	const BOX = 'flex flex-col gap-3 rounded-lg border border-border bg-card p-3';

	const sections = $derived(new Map(workflow.sections.map((s) => [s.id, s])));
	const solidSection = $derived(sections.get('solid-color'));
	const overlayTabs = $derived(
		OVERLAY_TABS.flatMap((tab) => {
			const section = sections.get(tab.section);
			return section ? [{ ...tab, available: section.available, reason: section.reason }] : [];
		})
	);

	// The pick survives bases that can't render it, but isn't shown as applied there.
	const solidPick = $derived(
		solidSection?.available
			? renderer.availableColors.find((c) => c.id === solidColor.selectedId)
			: undefined
	);
	const selection = $derived(artwork.selection);
	// An <img> never runs script inside the SVG, unlike inlining it with {@html}.
	const iconSrc = $derived(
		selection?.kind === 'icon' ? `data:image/svg+xml,${encodeURIComponent(selection.svg)}` : null
	);

	type Drill = 'emoji' | 'icon';

	let drill = $state<Drill | null>(null);
	let drillTrigger: HTMLElement | null = null;

	function openDrill(id: Drill, trigger: HTMLElement) {
		drillTrigger = trigger;
		drill = id;
	}

	async function closeDrill() {
		drill = null;
		// The trigger is inert until the close renders.
		await tick();
		drillTrigger?.focus();
	}

	// A layer the base can't render must not stay on.
	$effect(() => {
		const active = overlayTabs.find((tab) => tab.kind === artwork.kind);
		if (active && !active.available) untrack(() => artwork.setKind('none'));
	});
</script>

{#snippet choiceRow(
	name: string,
	label: string,
	value: string,
	choices: Choice[],
	onchange: (value: string) => void
)}
	<div class="flex items-center justify-between gap-3">
		<span id="{name}-label" class="text-sm text-muted-foreground">{label}</span>
		<RadioGroup.Root
			{value}
			onValueChange={onchange}
			aria-labelledby="{name}-label"
			class="flex flex-row gap-4"
		>
			{#each choices as choice (choice.value)}
				<div class="flex items-center gap-2">
					<RadioGroup.Item value={choice.value} id="{name}-{choice.value}" disabled={choice.disabled} />
					<label
						for="{name}-{choice.value}"
						title={choice.title}
						class={cn(
							'text-sm font-medium text-foreground',
							choice.disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'
						)}>{choice.label}</label
					>
				</div>
			{/each}
		</RadioGroup.Root>
	</div>
{/snippet}

{#snippet drillButton(id: Drill, label: string, preview: Snippet)}
	<Button
		variant="outline"
		class="w-full justify-start"
		onclick={(e: MouseEvent) => openDrill(id, e.currentTarget as HTMLElement)}
	>
		{@render preview()}
		<span class="truncate">{label}</span>
		<ChevronRightIcon class="ml-auto opacity-50" />
	</Button>
{/snippet}

{#snippet emojiPreview()}
	{#if selection?.kind === 'emoji'}
		<span class="text-base leading-none">{selection.emoji}</span>
	{/if}
{/snippet}

{#snippet iconPreview()}
	{#if iconSrc}
		<img src={iconSrc} alt="" class="size-4 dark:invert" />
	{/if}
{/snippet}

{#snippet anchorRow(name: string)}
	{#if features.experimental}
		{@render choiceRow(name, 'Anchor', artwork.anchorMode, ANCHOR_CHOICES, (v) => {
			if (v === 'inset' || v === 'centered') artwork.setAnchorMode(v);
		})}
	{/if}
{/snippet}

<!-- The picker panels cover the nearest positioned ancestor, which the host must provide. -->
<div class={cn('flex h-full flex-col gap-6 overflow-y-auto p-4', className)} inert={drill !== null}>
	{#if sections.has('source-image')}
		<section class="flex flex-col gap-2">
			<h2 class={HEADING}>Source image</h2>
			<SourceImagePicker />
		</section>
	{/if}

	{#if solidSection && solidSection.available}
		<section
			class="flex flex-col gap-2"
			title={solidSection.available ? undefined : solidSection.reason}
		>
			<h2 class={HEADING}>{solidSection.label}</h2>
			<ColorPicker
				label={solidSection.label}
				value={solidPick?.id ?? null}
				disabled={!solidSection.available}
				onchange={(color) => solidColor.select(color)}
			/>
		</section>
	{/if}

	<section class="flex flex-col gap-2">
		<h2 class={HEADING}>Overlay</h2>
		<Tabs.Root bind:value={() => artwork.kind, (kind) => artwork.setKind(kind as ArtworkKind)}>
			<Tabs.List class="w-full">
				<Tabs.Trigger value="none">None</Tabs.Trigger>
				{#each overlayTabs as tab (tab.kind)}
					<!-- Disabled triggers swallow pointer events, so the tooltip lives on a wrapper. -->
					<span class="flex h-full flex-1" title={tab.available ? undefined : tab.reason}>
						<Tabs.Trigger value={tab.kind} disabled={!tab.available}>{tab.label}</Tabs.Trigger>
					</span>
				{/each}
			</Tabs.List>

			<Tabs.Content value="none" class={BOX}>
				<p class="text-muted-foreground">Add a color dot, emoji or icon on top of the folder.</p>
			</Tabs.Content>

			<Tabs.Content value="colorDot" class={BOX}>
				{@render anchorRow('color-dot-anchor')}
				<ColorPicker
					label="Color dot"
					value={artwork.colorDotId}
					onchange={(color) => artwork.selectColorDot(color)}
				/>
			</Tabs.Content>

			<Tabs.Content value="emoji" class={BOX}>
				{@render anchorRow('emoji-anchor')}
				{@render drillButton(
					'emoji',
					selection?.kind === 'emoji' ? selection.data.name : 'Choose emoji…',
					emojiPreview
				)}
			</Tabs.Content>

			<Tabs.Content value="icon" class={BOX}>
				{@render choiceRow(
					'icon-placement',
					'Placement',
					artwork.iconPlacement,
					[
						{
							value: 'decal',
							label: 'Decal',
							disabled: !renderer.supportsDecal,
							title: renderer.rejections.decal
						},
						{ value: 'overlay', label: 'Overlay' }
					],
					(v) => {
						if (v === 'decal' || v === 'overlay') artwork.setPlacement(v);
					}
				)}
				{#if artwork.iconPlacement === 'overlay'}
					{@render anchorRow('icon-anchor')}
				{/if}
				{@render drillButton('icon', iconSrc ? 'Change icon…' : 'Choose icon…', iconPreview)}
			</Tabs.Content>
		</Tabs.Root>
	</section>
</div>

<!-- Mounting ~1,900 emoji is too slow to do on every open, so it happens behind the boot loader. -->
<PickerPanel title="Emoji" open={drill === 'emoji'} onclose={closeDrill}>
	<EmojiPicker.Root
		class="flex min-h-0 max-w-none flex-1 flex-col"
		onSelect={(emoji) => artwork.selectEmoji(emoji)}
		onSkinChange={(skin) => artwork.setEmojiSkin(skin)}
	>
		<EmojiPicker.Viewport class="flex min-h-0 flex-1 flex-col">
			<EmojiPicker.Search />
			<EmojiPicker.List class="h-auto max-h-none min-h-0 flex-1" />
			<EmojiPicker.Footer>
				<EmojiPicker.SkinToneSelector />
			</EmojiPicker.Footer>
		</EmojiPicker.Viewport>
	</EmojiPicker.Root>
</PickerPanel>

{#if drill === 'icon'}
	<PickerPanel title="Icon" open onclose={closeDrill}>
		<IconPicker
			collection="mdi"
			pageSize={30}
			class="min-h-0 flex-1"
			onselect={(svg) => artwork.selectIcon(svg)}
		/>
	</PickerPanel>
{/if}
