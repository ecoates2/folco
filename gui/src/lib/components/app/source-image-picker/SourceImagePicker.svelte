<script lang="ts">
	import { workflow } from '$lib/stores/workflow.svelte';

	let fileName = $state<string | null>(null);
	let error = $state<string | null>(null);

	async function handleChange(event: Event) {
		const file = (event.target as HTMLInputElement).files?.[0];
		if (!file) return;

		error = null;
		try {
			// SVG stays markup so it rasterizes cleanly at every platform size;
			// anything else is handed over as encoded bytes.
			if (file.type === 'image/svg+xml' || file.name.toLowerCase().endsWith('.svg')) {
				await workflow.useSvg(await file.text());
			} else {
				await workflow.useImage(new Uint8Array(await file.arrayBuffer()));
			}
			fileName = file.name;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}
</script>

<div class="flex flex-col gap-2">
	<label
		class="inline-flex w-fit cursor-pointer items-center rounded-md border border-border bg-background px-3 py-2 text-sm font-medium text-foreground hover:bg-muted"
	>
		Choose image…
		<input type="file" accept="image/*,.svg" class="hidden" onchange={handleChange} />
	</label>

	{#if fileName}
		<p class="text-sm text-muted-foreground">Using {fileName}</p>
	{:else}
		<p class="text-sm text-muted-foreground">
			Pick a PNG, JPEG, WebP or SVG to use as the folder icon.
		</p>
	{/if}

	{#if error}
		<p class="text-sm text-destructive">{error}</p>
	{/if}
</div>
