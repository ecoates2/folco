<script lang="ts">
	import { theme } from '$lib/stores/theme.svelte';
	import type { ThemePreference } from '$lib/stores/theme.svelte';
	import * as RadioGroup from '$lib/components/ui/radio-group';

	const options: ThemePreference[] = ['light', 'dark', 'system'];
</script>

<RadioGroup.Root
	value={theme.preference}
	onValueChange={(v: string) => theme.setPreference(v as ThemePreference)}
	class="flex flex-row gap-4"
>
	{#each options as pref}
		<div class="flex items-center gap-2">
			<RadioGroup.Item value={pref} />
			<label
				for="{pref}-theme"
				class="cursor-pointer text-sm font-medium text-foreground"
			>{pref[0].toUpperCase() + pref.slice(1)}</label>
		</div>
	{/each}
</RadioGroup.Root>
