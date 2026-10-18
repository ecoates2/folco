<script lang="ts">
	import type { FolderColorMetadata } from 'folco-renderer-wasm';
	import { renderer } from '$lib/stores/renderer.svelte';
	import { cn } from '$lib/utils';
	import XIcon from '@lucide/svelte/icons/x';

	interface Props {
		/** The currently selected color id, or null for no selection. */
		value?: string | null;
		/** Fired when a color is selected, or with `null` when "None" is picked. */
		onchange?: (color: FolderColorMetadata | null) => void;
		class?: string;
	}

	let { value = $bindable(null), onchange, class: className }: Props = $props();

	const colors = $derived(renderer.availableColors);

	const SWATCH =
		'size-7 rounded-full border-2 transition-transform hover:scale-110 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2';

	function select(color: FolderColorMetadata | null) {
		value = color?.id ?? null;
		onchange?.(color);
	}
</script>

<div
	class={cn('flex flex-wrap gap-1.5', className)}
	role="radiogroup"
	aria-label="Folder color"
>
	<button
		type="button"
		role="radio"
		aria-checked={value === null}
		aria-label="None"
		title="None"
		class={cn(
			SWATCH,
			'flex items-center justify-center bg-muted text-muted-foreground',
			value === null ? 'border-foreground scale-110' : 'border-border'
		)}
		onclick={() => select(null)}
	>
		<XIcon class="size-4" />
	</button>
	{#each colors as color (color.id)}
		{@const selected = value === color.id}
		<button
			type="button"
			role="radio"
			aria-checked={selected}
			aria-label={color.displayName}
			title={color.displayName}
			class={cn(SWATCH, selected ? 'border-foreground scale-110' : 'border-transparent')}
			style="background-color: rgb({color.r}, {color.g}, {color.b});"
			onclick={() => select(color)}
		></button>
	{/each}
</div>
