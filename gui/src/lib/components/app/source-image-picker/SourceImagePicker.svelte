<script lang="ts">
	import { open } from '@tauri-apps/plugin-dialog';
	import { Button } from '$lib/components/ui/button';
	import { errorDialog } from '$lib/stores/error-dialog.svelte';
	import { renderer } from '$lib/stores/renderer.svelte';
	import { workflow } from '$lib/stores/workflow.svelte';

	// Mirrors folco_core::ImageFile::supported_extensions; the backend rejects anything else.
	const EXTENSIONS = ['svg', 'png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif', 'tiff', 'tif'];

	async function pick() {
		try {
			const path = await open({
				title: 'Choose Image',
				multiple: false,
				directory: false,
				filters: [{ name: 'Images', extensions: EXTENSIONS }]
			});
			if (path) await workflow.useImageFile(path);
		} catch (e) {
			errorDialog.showError("Couldn't open the image", e);
		}
	}
</script>

<div class="flex flex-col gap-2">
	<Button variant="outline" class="w-fit" onclick={pick}>Choose image…</Button>

	{#if renderer.customImage}
		<p class="text-sm text-muted-foreground">Using {renderer.customImage.fileName}</p>
	{:else}
		<p class="text-sm text-muted-foreground">
			Pick a PNG, JPEG, WebP or SVG to use as the folder icon.
		</p>
	{/if}
</div>
