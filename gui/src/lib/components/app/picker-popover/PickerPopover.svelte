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
		/**
		 * Mounts the picker with the page and only hides it while closed, so an
		 * expensive picker is built once during boot instead of on every open.
		 */
		keepMounted?: boolean;
		children: Snippet;
	}

	let {
		title,
		label,
		preview,
		disabled = false,
		contentClass,
		keepMounted = false,
		children
	}: Props = $props();

	const panelClass = $derived(
		cn('max-h-(--bits-popover-content-available-height) w-auto overflow-y-auto', contentClass)
	);
</script>

{#snippet body()}
	<Popover.Header>
		<Popover.Title>{title}</Popover.Title>
	</Popover.Header>
	{@render children()}
{/snippet}

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
	{#if keepMounted}
		<Popover.Content side="bottom" align="end" class={panelClass} forceMount>
			{#snippet child({ wrapperProps, props, open })}
				<div {...wrapperProps} hidden={!open}>
					<div {...props}>
						{@render body()}
					</div>
				</div>
			{/snippet}
		</Popover.Content>
	{:else}
		<Popover.Content side="bottom" align="end" class={panelClass}>
			{@render body()}
		</Popover.Content>
	{/if}
</Popover.Root>
