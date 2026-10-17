<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as Popover from '$lib/components/ui/popover';
	import { Button } from '$lib/components/ui/button';
	import { cn } from '$lib/utils';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';

	interface Props {
		/** Heading of the floating panel. */
		title: string;
		/** Text on the trigger button. */
		label: string;
		/** Compact rendering of the current pick, shown before the label. */
		preview?: Snippet;
		disabled?: boolean;
		/** Classes for the floating panel, which sizes itself to its picker. */
		contentClass?: string;
		children: Snippet;
	}

	let { title, label, preview, disabled = false, contentClass, children }: Props = $props();
</script>

<Popover.Root>
	<Popover.Trigger {disabled}>
		{#snippet child({ props })}
			<Button {...props} variant="outline" class="w-full justify-start">
				{@render preview?.()}
				<span class="truncate">{label}</span>
				<ChevronDownIcon class="ml-auto opacity-50" />
			</Button>
		{/snippet}
	</Popover.Trigger>
	<!-- Opens downward so the preview to the left stays visible while picking. -->
	<Popover.Content
		side="bottom"
		align="end"
		class={cn(
			'max-h-(--bits-popover-content-available-height) w-auto overflow-y-auto',
			contentClass
		)}
	>
		<Popover.Header>
			<Popover.Title>{title}</Popover.Title>
		</Popover.Header>
		{@render children()}
	</Popover.Content>
</Popover.Root>
