<script lang="ts">
	import type { FolderColorMetadata } from 'folco-renderer-wasm';
	import { renderer } from '$lib/stores/renderer.svelte';
	import { cn } from '$lib/utils';

	interface Props {
		/** The currently selected color id, or null for no selection. */
		value?: string | null;
		/** Fired when a color is selected. */
		onchange?: (color: FolderColorMetadata) => void;
		class?: string;
	}

	let { value = $bindable(null), onchange, class: className }: Props = $props();

	const colors = $derived(renderer.availableColors);

	function select(color: FolderColorMetadata) {
		value = color.id;
		onchange?.(color);
	}
</script>

<div
	class={cn('flex flex-wrap gap-1.5', className)}
	role="radiogroup"
	aria-label="Folder color"
>
	{#each colors as color (color.id)}
		{@const selected = value === color.id}
		<button
			type="button"
			role="radio"
			aria-checked={selected}
			aria-label={color.displayName}
			title={color.displayName}
			class={cn(
				'size-7 rounded-full border-2 transition-transform hover:scale-110 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
				selected ? 'border-foreground scale-110' : 'border-transparent'
			)}
			style="background-color: rgb({color.r}, {color.g}, {color.b});"
			onclick={() => select(color)}
		></button>
	{/each}
</div>
