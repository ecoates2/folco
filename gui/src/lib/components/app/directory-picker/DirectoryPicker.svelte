<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { cn } from '$lib/utils';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import XIcon from '@lucide/svelte/icons/x';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
	import { onMount } from 'svelte';

	interface Props {
		directories: string[];
		selectedIndex?: number | null;
		onAdd?: () => void;
		onRemove?: (index: number) => void;
		onClearAll?: () => void;
		onSelect?: (index: number) => void;
		onDropPaths?: (paths: string[]) => void;
		class?: string;
	}

	let {
		directories = $bindable([]),
		selectedIndex = $bindable(null),
		onAdd,
		onRemove,
		onClearAll,
		onSelect,
		onDropPaths,
		class: className
	}: Props = $props();

	let isDragging = $state(false);

	onMount(() => {
		const appWindow = getCurrentWebviewWindow();
		const unlisten = appWindow.onDragDropEvent((event) => {
			if (event.payload.type === 'over') {
				isDragging = true;
			} else if (event.payload.type === 'leave') {
				isDragging = false;
			} else if (event.payload.type === 'drop') {
				isDragging = false;
				const paths = event.payload.paths;
				if (paths.length > 0 && onDropPaths) {
					onDropPaths(paths);
				}
			}
		});

		return () => {
			unlisten.then((fn) => fn());
		};
	});

	function handleItemClick(index: number) {
		selectedIndex = index;
		onSelect?.(index);
	}

	function handleRemove() {
		if (selectedIndex !== null && selectedIndex >= 0) {
			onRemove?.(selectedIndex);
			selectedIndex = null;
		}
	}

	function handleKeyDown(event: KeyboardEvent, index: number) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			handleItemClick(index);
		}
	}
</script>

<div class={cn('flex flex-col gap-3', className)}>
	<div class="text-lg font-semibold text-foreground">Folder(s)</div>

	<!-- Directory List / Drop Zone -->
	<div
		role="listbox"
		tabindex="0"
		aria-label="Selected directories"
		class={cn(
			'min-h-40 rounded-md border-2 border-dashed bg-background p-2 transition-colors',
			isDragging
				? 'border-primary bg-primary/5'
				: 'border-input hover:border-muted-foreground/50',
			directories.length === 0 && 'flex items-center justify-center'
		)}
	>
		{#if directories.length === 0}
			<div class="flex flex-col items-center gap-2 text-muted-foreground">
				<FolderIcon class="size-8 opacity-50" />
				<span class="text-sm">Drop folders here or click Add</span>
			</div>
		{:else}
			<div class="flex flex-col gap-1">
				{#each directories as dir, index}
					<div
						role="option"
						tabindex="0"
						aria-selected={selectedIndex === index}
						onclick={() => handleItemClick(index)}
						onkeydown={(e) => handleKeyDown(e, index)}
						class={cn(
							'flex cursor-pointer items-center gap-2 rounded px-2 py-1.5 text-sm transition-colors',
							selectedIndex === index
								? 'bg-primary text-primary-foreground'
								: 'hover:bg-accent'
						)}
					>
						<FolderIcon class="size-4 shrink-0" />
						<span class="truncate">{dir}</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Action Buttons -->
	<div class="flex gap-2">
		<Button variant="default" size="sm" onclick={onAdd} class="flex-1">
			Add
		</Button>
		<Button
			variant="default"
			size="sm"
			onclick={handleRemove}
			disabled={selectedIndex === null}
			class="flex-1"
		>
			Remove
		</Button>
		<Button
			variant="default"
			size="sm"
			onclick={onClearAll}
			disabled={directories.length === 0}
			class="flex-1"
		>
			Clear All
		</Button>
	</div>
</div>
