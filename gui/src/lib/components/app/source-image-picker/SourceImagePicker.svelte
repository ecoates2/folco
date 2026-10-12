<script lang="ts">
	import { open } from '@tauri-apps/plugin-dialog';
	import { Button } from '$lib/components/ui/button';
	import { errorDialog } from '$lib/stores/error-dialog.svelte';
	import { platform } from '$lib/stores/platform.svelte';
	import { renderer } from '$lib/stores/renderer.svelte';
	import { workflow } from '$lib/stores/workflow.svelte';

	const support = $derived(platform.customImageSupport);

	const hint = $derived.by(() => {
		if (!support) return 'Pick an image to use as the folder icon.';
		if (!support.write.raster) return 'Pick an SVG to use as the folder icon.';
		if (!support.write.vector) {
			return 'Pick an image to use as the folder icon. SVGs are converted to PNG on this system.';
		}
		return 'Pick a PNG, JPEG, WebP or SVG to use as the folder icon.';
	});

	async function pick() {
		if (!support) return;
		try {
			const path = await open({
				title: 'Choose Image',
				multiple: false,
				directory: false,
				filters: [{ name: 'Images', extensions: support.extensions }]
			});
			if (path) await workflow.useImageFile(path);
		} catch (e) {
			errorDialog.showError("Couldn't open the image", e);
		}
	}
</script>

<div class="flex flex-col gap-2">
	<Button variant="outline" class="w-fit" onclick={pick} disabled={!support}>Choose image…</Button>

	{#if renderer.customImage}
		<p class="text-sm text-muted-foreground">Using {renderer.customImage.fileName}</p>
	{:else}
		<p class="text-sm text-muted-foreground">{hint}</p>
	{/if}
</div>
