<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { ThemeSelector } from '$lib/components/app/theme-selector';

	interface Props {
		open?: boolean;
	}

	let { open = $bindable(false) }: Props = $props();
</script>

<Dialog.Root bind:open>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Settings</Dialog.Title>
		</Dialog.Header>

		<div role="group" aria-labelledby="settings-theme" class="flex flex-col gap-2">
			<span id="settings-theme" class="text-sm font-medium text-foreground">Theme</span>
			<ThemeSelector />
		</div>
	</Dialog.Content>
</Dialog.Root>
