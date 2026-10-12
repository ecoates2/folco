<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import type { ErrorReport } from '$lib/stores/error-dialog.svelte';

	interface Props {
		report: ErrorReport | null;
		onDismiss: () => void;
	}

	let { report, onDismiss }: Props = $props();

	// Keeps the last report rendered while the close animation plays.
	let last: ErrorReport | null = null;
	const shown = $derived.by(() => (last = report ?? last));
</script>

<Dialog.Root
	bind:open={() => report !== null, (open) => {
		if (!open) onDismiss();
	}}
>
	<Dialog.Content class={shown?.failures?.length ? 'sm:max-w-2xl' : undefined}>
		{#if shown}
			<Dialog.Header>
				<Dialog.Title>{shown.title}</Dialog.Title>
				<Dialog.Description class="break-words whitespace-pre-wrap">
					{shown.message}
				</Dialog.Description>
			</Dialog.Header>

			{#if shown.failures?.length}
				<div class="max-h-72 overflow-auto rounded-md border">
					<table class="w-full text-sm">
						<thead class="sticky top-0 bg-muted text-left text-muted-foreground">
							<tr>
								<th class="px-3 py-2 font-medium">Folder</th>
								<th class="px-3 py-2 font-medium">Error</th>
							</tr>
						</thead>
						<tbody>
							{#each shown.failures as failure}
								<tr class="border-t align-top">
									<td class="w-1/2 px-3 py-2 font-mono text-xs break-all">{failure.path}</td>
									<td class="px-3 py-2 break-words">{failure.error}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}

			<Dialog.Footer>
				<Button onclick={onDismiss}>OK</Button>
			</Dialog.Footer>
		{/if}
	</Dialog.Content>
</Dialog.Root>
