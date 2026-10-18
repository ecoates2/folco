<script lang="ts">
	import { untrack } from 'svelte';
	import type { FolderColorMetadata } from 'folco-renderer-wasm';
	import { artwork, type ArtworkKind } from '$lib/stores/artwork.svelte';
	import { features } from '$lib/stores/features.svelte';
	import { renderer } from '$lib/stores/renderer.svelte';
	import { solidColor } from '$lib/stores/solid-color.svelte';
	import { workflow } from '$lib/stores/workflow.svelte';
	import type { SectionId } from '$lib/workflows/types';
	import { cn } from '$lib/utils';

	import { ColorPicker } from '$lib/components/app/color-picker';
	import { IconPicker } from '$lib/components/app/icon-picker';
	import { PickerPopover } from '$lib/components/app/picker-popover';
	import { SourceImagePicker } from '$lib/components/app/source-image-picker';

	import * as EmojiPicker from '$lib/components/ui/emoji-picker';
	import * as RadioGroup from '$lib/components/ui/radio-group';
	import { Switch } from '$lib/components/ui/switch';
	import * as Tabs from '$lib/components/ui/tabs';

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

	const solidPick = $derived(
		renderer.availableColors.find((c) => c.id === solidColor.selectedId)
	);
	const selection = $derived(artwork.selection);
	// An <img> never runs script inside the SVG, unlike inlining it with {@html}.
	const iconSrc = $derived(
		selection?.kind === 'icon' ? `data:image/svg+xml,${encodeURIComponent(selection.svg)}` : null
	);

	// A layer the base can't render must not stay on.
	$effect(() => {
		if (solidSection && !solidSection.available && solidColor.enabled) {
			untrack(() => solidColor.setEnabled(false));
		}
	});

	$effect(() => {
		const active = overlayTabs.find((tab) => tab.kind === artwork.kind);
		if (active && !active.available) untrack(() => artwork.setKind('none'));
	});
</script>

{#snippet swatch(color: FolderColorMetadata | undefined)}
	<span
		class="size-4 shrink-0 rounded-full border border-border"
		style:background-color={color ? `rgb(${color.r}, ${color.g}, ${color.b})` : 'transparent'}
	></span>
{/snippet}

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

{#snippet anchorRow(name: string)}
	{#if features.experimental}
		{@render choiceRow(name, 'Anchor', artwork.anchorMode, ANCHOR_CHOICES, (v) => {
			if (v === 'inset' || v === 'centered') artwork.setAnchorMode(v);
		})}
	{/if}
{/snippet}

<div class={cn('flex flex-col gap-6', className)}>
	{#if sections.has('source-image')}
		<section class="flex flex-col gap-2">
			<h2 class={HEADING}>Source image</h2>
			<SourceImagePicker />
		</section>
	{/if}

	{#if solidSection}
		<section
			class="flex flex-col gap-2"
			title={solidSection.available ? undefined : solidSection.reason}
		>
			<div class="flex items-center justify-between">
				<h2 id="solid-color-heading" class={HEADING}>{solidSection.label}</h2>
				<Switch
					bind:checked={() => solidColor.enabled, (on) => solidColor.setEnabled(on)}
					disabled={!solidSection.available}
					aria-labelledby="solid-color-heading"
				/>
			</div>
			<PickerPopover
				title={solidSection.label}
				label={solidPick?.displayName ?? 'Choose color…'}
				disabled={!solidSection.available}
				contentClass="w-64"
			>
				{#snippet preview()}
					{@render swatch(solidPick)}
				{/snippet}
				<ColorPicker value={solidColor.selectedId} onchange={(color) => solidColor.select(color)} />
			</PickerPopover>
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
				<PickerPopover
					title="Color dot"
					label={selection?.kind === 'colorDot' ? selection.color.displayName : 'Choose color…'}
					contentClass="w-64"
				>
					{#snippet preview()}
						{@render swatch(selection?.kind === 'colorDot' ? selection.color : undefined)}
					{/snippet}
					<ColorPicker value={artwork.colorDotId} onchange={(color) => artwork.selectColorDot(color)} />
				</PickerPopover>
			</Tabs.Content>

			<Tabs.Content value="emoji" class={BOX}>
				{@render anchorRow('emoji-anchor')}
				<!-- Mounting ~1,900 emoji is too slow to do on every open, so it happens behind the boot loader. -->
				<PickerPopover
					title="Emoji"
					label={selection?.kind === 'emoji' ? selection.data.name : 'Choose emoji…'}
					keepMounted
				>
					{#snippet preview()}
						{#if selection?.kind === 'emoji'}
							<span class="text-base leading-none">{selection.emoji}</span>
						{/if}
					{/snippet}
					<EmojiPicker.Root
						onSelect={(emoji) => artwork.selectEmoji(emoji)}
						onSkinChange={(skin) => artwork.setEmojiSkin(skin)}
					>
						<EmojiPicker.Viewport>
							<EmojiPicker.Search />
							<EmojiPicker.List class="h-64" />
							<EmojiPicker.Footer>
								<EmojiPicker.SkinToneSelector />
							</EmojiPicker.Footer>
						</EmojiPicker.Viewport>
					</EmojiPicker.Root>
				</PickerPopover>
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
				<PickerPopover title="Icon" label={iconSrc ? 'Change icon…' : 'Choose icon…'}>
					{#snippet preview()}
						{#if iconSrc}
							<img src={iconSrc} alt="" class="size-4 dark:invert" />
						{/if}
					{/snippet}
					<!-- Search is hidden: it's currently broken. -->
					<IconPicker
						collection="mdi"
						hideSearch
						pageSize={30}
						height="24rem"
						class="block w-[35rem]"
						onselect={(svg) => artwork.selectIcon(svg)}
					/>
				</PickerPopover>
			</Tabs.Content>
		</Tabs.Root>
	</section>
</div>
