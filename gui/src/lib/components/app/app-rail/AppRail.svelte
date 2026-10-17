<script lang="ts">
	import { mergeProps } from 'bits-ui';
	import { workflow } from '$lib/stores/workflow.svelte';
	import { WORKFLOWS } from '$lib/workflows/definitions';
	import type { WorkflowId } from '$lib/workflows/types';
	import { AboutDialog } from '$lib/components/app/about-dialog';
	import { SettingsDialog } from '$lib/components/app/settings-dialog';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { Button } from '$lib/components/ui/button';
	import { cn } from '$lib/utils';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import ImageIcon from '@lucide/svelte/icons/image';
	import InfoIcon from '@lucide/svelte/icons/info';
	import SettingsIcon from '@lucide/svelte/icons/settings';

	interface Props {
		class?: string;
	}

	let { class: className }: Props = $props();

	const WORKFLOW_ICONS: Record<WorkflowId, typeof FolderIcon> = {
		folder: FolderIcon,
		'custom-image': ImageIcon
	};

	const WORKFLOW_IDS = Object.keys(WORKFLOWS) as WorkflowId[];

	let settingsOpen = $state(false);
	let aboutOpen = $state(false);
</script>

{#snippet railButton(label: string, Icon: typeof FolderIcon, onclick: () => void, active?: boolean)}
	<Tooltip.Root>
		<Tooltip.Trigger>
			{#snippet child({ props })}
				<Button
					{...mergeProps(props, { onclick })}
					variant="ghost"
					size="icon"
					aria-label={label}
					aria-pressed={active}
					class={cn(active && 'bg-muted text-foreground')}
				>
					<Icon class="size-5" />
				</Button>
			{/snippet}
		</Tooltip.Trigger>
		<Tooltip.Content side="right">{label}</Tooltip.Content>
	</Tooltip.Root>
{/snippet}

<Tooltip.Provider>
	<nav
		aria-label="Main"
		class={cn('flex flex-col items-center gap-1 border-r border-border bg-muted/30 p-1.5', className)}
	>
		{#each WORKFLOW_IDS as id (id)}
			{@render railButton(
				WORKFLOWS[id].title,
				WORKFLOW_ICONS[id],
				() => {
					// Re-activating drops a loaded custom image, so only act on a real switch.
					if (workflow.id !== id) workflow.activate(id);
				},
				workflow.id === id
			)}
		{/each}

		<div class="mt-auto flex flex-col gap-1">
			{@render railButton('Settings', SettingsIcon, () => (settingsOpen = true))}
			{@render railButton('About', InfoIcon, () => (aboutOpen = true))}
		</div>
	</nav>
</Tooltip.Provider>

<SettingsDialog bind:open={settingsOpen} />
<AboutDialog bind:open={aboutOpen} />
