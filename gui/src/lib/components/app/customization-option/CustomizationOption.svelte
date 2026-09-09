<script lang="ts">
	import { cn } from '$lib/utils';
	import type { Snippet } from 'svelte';

	interface Props {
		/** Section label shown in the header. */
		label: string;
		/** Whether this customization is currently enabled. */
		enabled?: boolean;
		/** Whether the active icon medium supports this customization. */
		disabled?: boolean;
		/** Explains why the option is unavailable, shown when disabled. */
		disabledReason?: string;
		/** Fired when the toggle changes. */
		onToggle?: (enabled: boolean) => void;
		/** The customization UI to render inside the section. */
		children: Snippet;
		class?: string;
	}

	let {
		label,
		enabled = $bindable(false),
		disabled = false,
		disabledReason,
		onToggle,
		children,
		class: className
	}: Props = $props();

	// A disabled option must not stay on: the medium can't honour it.
	$effect(() => {
		if (disabled && enabled) {
			enabled = false;
			onToggle?.(false);
		}
	});

	function toggle() {
		if (disabled) return;
		enabled = !enabled;
		onToggle?.(enabled);
	}
</script>

<section class={cn('rounded-lg border border-border bg-card', disabled && 'opacity-50', className)}>
	<button
		type="button"
		{disabled}
		title={disabled ? disabledReason : undefined}
		class="flex w-full items-center justify-between px-4 py-3 text-left disabled:cursor-not-allowed"
		onclick={toggle}
	>
		<span class="text-sm font-medium text-card-foreground">{label}</span>

		<!-- Toggle switch -->
		<span
			class={cn(
				'relative inline-flex h-5 w-9 shrink-0 rounded-full border-2 border-transparent transition-colors',
				disabled ? 'cursor-not-allowed' : 'cursor-pointer',
				enabled ? 'bg-primary' : 'bg-input'
			)}
			role="switch"
			aria-checked={enabled}
			aria-disabled={disabled}
		>
			<span
				class={cn(
					'pointer-events-none block size-4 rounded-full bg-background shadow-sm ring-0 transition-transform',
					enabled ? 'translate-x-4' : 'translate-x-0'
				)}
			></span>
		</span>
	</button>

	<div class="border-t border-border px-4 py-3" class:hidden={!enabled || disabled}>
		{@render children()}
	</div>
</section>
