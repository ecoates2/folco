<script lang="ts">
	import { onMount } from 'svelte';
	import { workflow } from '$lib/stores/workflow.svelte';
	import { directories } from '$lib/stores/directories.svelte';
	import { folderOperations, type FolderOperation } from '$lib/stores/folder-operations.svelte';
	import { cn } from '$lib/utils';

	import { AppRail } from '$lib/components/app/app-rail';
	import { CustomizationPanel } from '$lib/components/app/customization-panel';
	import { DirectoryPicker } from '$lib/components/app/directory-picker';
	import { IconPreview } from '$lib/components/app/icon-preview';

	import { Button } from '$lib/components/ui/button';
	// Not ui/spinner: its null-forwarding renders an invisible icon.
	import Loader2Icon from '@lucide/svelte/icons/loader-2';

	interface Props {
		class?: string;
	}

	let { class: className }: Props = $props();

	onMount(() => workflow.activate('folder'));

	const OPERATION_LABELS: Record<FolderOperation, { idle: string; running: string; done: string }> = {
		apply: { idle: 'Apply', running: 'Applying', done: 'applied' },
		reset: { idle: 'Reset', running: 'Resetting', done: 'reset' }
	};
</script>

{#snippet operationButton(
	operation: FolderOperation,
	blockedReason: string | null,
	variant: 'default' | 'outline',
	onclick: () => void
)}
	<!-- Disabled buttons swallow pointer events, so the tooltip lives on a wrapper. -->
	<span title={blockedReason ?? undefined}>
		<Button {variant} {onclick} disabled={blockedReason !== null}>
			{#if folderOperations.status === 'running' && folderOperations.operation === operation}
				<Loader2Icon class="size-4 animate-spin" />
				{OPERATION_LABELS[operation].running} {folderOperations.processed}/{folderOperations.total}
			{:else}
				{OPERATION_LABELS[operation].idle}
			{/if}
		</Button>
	</span>
{/snippet}

<div class={cn('grid h-full min-h-0 grid-cols-[auto_1fr] bg-background', className)}>
	<AppRail />

	<div class="grid min-h-0 grid-rows-[1fr_auto]">
		<!-- The preview hugs its content; the sidebar takes all remaining width. -->
		<div class="grid min-h-0 grid-cols-[min-content_minmax(0,1fr)]">
			<section aria-label="Preview" class="flex min-h-0 items-center justify-center overflow-hidden p-6">
				<IconPreview />
			</section>

			<aside aria-label="Customization" class="relative min-h-0 border-l border-border">
				<CustomizationPanel />
			</aside>
		</div>

		<section class="flex h-60 flex-col gap-3 border-t border-border p-4">
			<DirectoryPicker
				bind:directories={directories.paths}
				bind:selectedIndex={directories.selectedIndex}
				onAdd={() => directories.browse()}
				onRemove={(i) => directories.remove(i)}
				onClearAll={() => directories.clear()}
				onDropPaths={(paths) => directories.add(paths)}
				class="min-h-0 flex-1"
			/>

			<div class="flex items-center gap-3">
				{#if folderOperations.status === 'done' && !folderOperations.error}
					<span class="text-sm text-muted-foreground">
						{folderOperations.succeeded}
						{OPERATION_LABELS[folderOperations.operation].done}{folderOperations.failures.length
							? `, ${folderOperations.failures.length} failed`
							: ''}
					</span>
				{/if}

				<div class="ml-auto flex items-center gap-2">
					{@render operationButton('reset', folderOperations.resetBlockedReason, 'outline', () =>
						folderOperations.reset()
					)}
					{@render operationButton('apply', folderOperations.applyBlockedReason, 'default', () =>
						folderOperations.apply()
					)}
				</div>
			</div>
		</section>
	</div>
</div>
