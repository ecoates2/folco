<script lang="ts">
	import { getVersion } from '@tauri-apps/api/app';
	import * as Dialog from '$lib/components/ui/dialog';
	import { onMount } from 'svelte';

	interface Props {
		open?: boolean;
	}

	let { open = $bindable(false) }: Props = $props();

	let version = $state<string | null>(null);

	onMount(() => {
		// Rejects outside Tauri (browser dev server); the version line is just omitted.
		getVersion().then(
			(v) => (version = v),
			() => {}
		);
	});
</script>

<Dialog.Root bind:open>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Folco</Dialog.Title>
			<Dialog.Description>
				Customize folder icons with colors, emoji and icons.
			</Dialog.Description>
		</Dialog.Header>

		{#if version}
			<p class="text-sm text-muted-foreground">Version {version}</p>
		{/if}
	</Dialog.Content>
</Dialog.Root>
