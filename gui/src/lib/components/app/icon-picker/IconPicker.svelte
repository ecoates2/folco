<script lang="ts">
	import 'iconify-picker';

	interface Props {
		collection?: string;
		hideSearch?: boolean;
		pageSize?: number;
		buttonLabel?: string;
		filter?: string;
		mode?: 'inline' | 'popover';
		iconTheme?: 'auto' | 'light' | 'dark';
		height?: string;
		class?: string;
		oniconselected?: (event: CustomEvent) => void;
	}

	let {
		collection = 'mdi',
		hideSearch = false,
		pageSize = 60,
		buttonLabel = 'Choose Icon',
		filter = '',
		mode = 'inline',
		iconTheme = 'auto',
		height = '100%',
		class: className,
		oniconselected
	}: Props = $props();
</script>

<iconify-picker
	class={className}
	{collection}
	{filter}
	{mode}
	{height}
	page-size={pageSize}
	button-label={buttonLabel}
	theme={iconTheme}
	hide-search={hideSearch ? '' : undefined}
	onicon-selected={oniconselected}
></iconify-picker>

<style>
	/* The picker reads these off its host and they inherit into its shadow root,
	   so shadcn tokens re-resolve automatically when `.dark` toggles. */
	iconify-picker {
		/* Defeat the component's own `color-scheme: dark light`, which pins its native
		   <select> popup to the OS scheme instead of ours. */
		color-scheme: inherit;
		--bg: var(--card);
		--text: var(--card-foreground);
		--hover: var(--accent);
		--header-bg: var(--card);
		--footer-bg: var(--card);
		--input-bg: var(--background);
		--input-text: var(--foreground);
		--icon-color: var(--foreground);
	}
</style>
