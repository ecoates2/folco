<script lang="ts">
	import { renderer } from '$lib/stores/renderer.svelte';
	import { cn } from '$lib/utils';

	interface Props {
		/** Fixed display size of the preview area in pixels. */
		displaySize?: number;
		class?: string;
	}

	let { displaySize = 256, class: className }: Props = $props();

	let canvas = $state<HTMLCanvasElement | null>(null);
	let container = $state<HTMLDivElement | null>(null);
	let selectedSize = $state<number>(0);

	function render() {
		if (!canvas || renderer.status !== 'ready') return;

		try {
			const [width, height] = renderer.getRenderedDimensions(selectedSize);
			canvas.width = width;
			canvas.height = height;

			renderer.renderToCanvas(canvas, selectedSize);
		} catch (e) {
			console.error('Render failed:', e);
		}
	}

	// Re-render when the renderer becomes ready, size changes, or customization state changes
	$effect(() => {
		// Track the version counter so any customization change triggers a re-render
		const _version = renderer.version;

		if (renderer.status === 'ready' && canvas && selectedSize > 0) {
			render();
		}
	});

	// Default to the largest available size
	$effect(() => {
		const sizes = renderer.availableSizes;
		if (sizes.length > 0 && selectedSize === 0) {
			selectedSize = sizes[sizes.length - 1];
		}
	});

	function handleWheel(e: WheelEvent) {
		const sizes = renderer.availableSizes;
		if (sizes.length <= 1) return;

		e.preventDefault();
		const currentIndex = sizes.indexOf(selectedSize);
		if (currentIndex === -1) return;

		if (e.deltaY < 0 && currentIndex < sizes.length - 1) {
			selectedSize = sizes[currentIndex + 1];
		} else if (e.deltaY > 0 && currentIndex > 0) {
			selectedSize = sizes[currentIndex - 1];
		}
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	bind:this={container}
	onwheel={handleWheel}
	class={cn('inline-flex flex-col items-center gap-3 rounded-lg border border-border bg-background p-4', className)}
>
	<div
		class="flex items-center justify-center"
		style="width: {displaySize}px; height: {displaySize}px;"
	>
		<canvas
			bind:this={canvas}
			style="max-width: {displaySize}px; max-height: {displaySize}px; width: auto; height: auto;"
		></canvas>
	</div>

	{#if renderer.availableSizes.length > 1}
		<select
			bind:value={selectedSize}
			class="rounded-md border border-border bg-background px-2 py-1 text-sm text-foreground"
		>
			{#each [...renderer.availableSizes].reverse() as size}
				<option value={size}>{size} × {size}</option>
			{/each}
		</select>
	{/if}
</div>
