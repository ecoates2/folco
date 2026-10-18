<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';

	interface Props {
		title: string;
		open: boolean;
		onclose: () => void;
		children: Snippet;
	}

	let { title, open, onclose, children }: Props = $props();

	let back = $state<HTMLElement | null>(null);

	$effect(() => {
		if (open) back?.focus();
	});
</script>

<!-- Covers its positioned parent, so the picker takes over the sidebar instead of the preview. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions (Escape bubbles up from the picker's controls) -->
<div
	role="region"
	aria-label={title}
	hidden={!open}
	class="absolute inset-0 z-10 flex flex-col gap-3 bg-background p-4"
	onkeydown={(e) => {
		if (e.key === 'Escape') onclose();
	}}
>
	<header class="flex items-center gap-1">
		<Button bind:ref={back} variant="ghost" size="icon-sm" aria-label="Back" onclick={onclose}>
			<ChevronLeftIcon />
		</Button>
		<h2 class="text-sm font-semibold">{title}</h2>
	</header>
	<div class="flex min-h-0 flex-1 flex-col">
		{@render children()}
	</div>
</div>
