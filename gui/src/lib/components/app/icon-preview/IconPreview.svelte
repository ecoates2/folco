<script lang="ts">
	import { renderer } from '$lib/stores/renderer.svelte';
	import { cn } from '$lib/utils';
	import * as Select from '$lib/components/ui/select';
	import { Spinner } from '$lib/components/ui/spinner';

	interface Props {
		/** Fixed display size of the preview area in pixels. */
		displaySize?: number;
		class?: string;
	}

	let { displaySize = 256, class: className }: Props = $props();

	let canvas = $state<HTMLCanvasElement | null>(null);
	let container = $state<HTMLDivElement | null>(null);
	let selectedSize = $state<number>(0);

	// `renderer.status === 'ready'` only means the data arrived; pixels land a few
	// effect flushes later, once `selectedSize` is set and the canvas is bound.
	let hasPainted = $state(false);

	function render() {
		if (!canvas || renderer.status !== 'ready') return;

		try {
			renderer.renderToCanvas(canvas, selectedSize);
			hasPainted = true;
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
		class="relative flex items-center justify-center"
		style="width: {displaySize}px; height: {displaySize}px;"
	>
		<!-- Kept mounted so `bind:this` resolves before the render effect runs. -->
		<canvas
			bind:this={canvas}
			class:invisible={!hasPainted}
			style="max-width: {displaySize}px; max-height: {displaySize}px; width: auto; height: auto;"
		></canvas>

		{#if !hasPainted}
			<div class="absolute inset-0 flex items-center justify-center">
				{#if renderer.status === 'error'}
					<p class="text-muted-foreground px-4 text-center text-sm">Preview unavailable</p>
				{:else}
				<!-- TODO: Revisit broken spinner rendering: renders in DOM but is invisible -->
					<Spinner class="text-muted-foreground size-8" />
				{/if}
			</div>
		{/if}
	</div>

	{#if renderer.availableSizes.length > 1}
		<Select.Root
			type="single"
			value={String(selectedSize)}
			onValueChange={(v) => (selectedSize = Number(v))}
		>
			<Select.Trigger size="sm" aria-label="Preview resolution">
				{selectedSize} × {selectedSize}
			</Select.Trigger>
			<Select.Content>
				{#each [...renderer.availableSizes].reverse() as size (size)}
					<Select.Item value={String(size)} label="{size} × {size}" />
				{/each}
			</Select.Content>
		</Select.Root>
	{/if}
</div>
