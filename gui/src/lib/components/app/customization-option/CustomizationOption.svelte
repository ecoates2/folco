<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';

	interface Props {
		/** Section label shown in the header. */
		label: string;
		/** Whether this customization is currently enabled. */
		enabled?: boolean;
		/** Fired when the toggle changes. */
		onToggle?: (enabled: boolean) => void;
		/** The customization UI to render inside the section. */
		children: Snippet;
		class?: string;
	}

	let {
		label,
		enabled = $bindable(false),
		onToggle,
		children,
		class: className
	}: Props = $props();

	function toggle() {
		enabled = !enabled;
		onToggle?.(enabled);
	}
</script>

<section class={cn('rounded-lg border border-border bg-card', className)}>
	<button
		type="button"
		class="flex w-full items-center justify-between px-4 py-3 text-left"
		onclick={toggle}
	>
		<span class="text-sm font-medium text-card-foreground">{label}</span>

		<!-- Toggle switch -->
		<span
			class={cn(
				'relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors',
				enabled ? 'bg-primary' : 'bg-input'
			)}
			role="switch"
			aria-checked={enabled}
		>
			<span
				class={cn(
					'pointer-events-none block size-4 rounded-full bg-background shadow-sm ring-0 transition-transform',
					enabled ? 'translate-x-4' : 'translate-x-0'
				)}
			></span>
		</span>
	</button>

	<div class="border-t border-border px-4 py-3" class:hidden={!enabled}>
		{@render children()}
	</div>
</section>
