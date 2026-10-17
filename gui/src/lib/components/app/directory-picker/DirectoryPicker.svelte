<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { cn } from '$lib/utils';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import MinusIcon from '@lucide/svelte/icons/minus';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import XIcon from '@lucide/svelte/icons/x';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
	import { stat } from '@tauri-apps/plugin-fs';
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
		// Native drag-and-drop only exists under Tauri; the browser dev server has no backend.
		let unlisten: Promise<() => void> | null = null;
		try {
			unlisten = getCurrentWebviewWindow().onDragDropEvent((event) => {
				if (event.payload.type === 'over') {
					isDragging = true;
				} else if (event.payload.type === 'leave') {
					isDragging = false;
				} else if (event.payload.type === 'drop') {
					isDragging = false;
					void addDroppedDirectories(event.payload.paths);
				}
			});
		} catch {
			unlisten = null;
		}

		return () => {
			unlisten?.then((fn) => fn()).catch(() => {});
		};
	});

	// The fs plugin scopes dropped paths itself, so `fs:allow-stat` needs no static scope.
	async function addDroppedDirectories(paths: string[]) {
		const isDir = await Promise.all(
			paths.map((p) => stat(p).then((info) => info.isDirectory, () => false))
		);
		const dirs = paths.filter((_, i) => isDir[i]);
		if (dirs.length > 0) onDropPaths?.(dirs);
	}

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

<div class={cn('flex flex-col gap-2', className)}>
	<div class="flex items-center gap-2">
		<h2 class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
			Folders{#if directories.length > 0}&nbsp;({directories.length}){/if}
		</h2>

		<div class="ml-auto flex gap-1.5">
			<Button variant="outline" size="sm" onclick={onAdd}>
				<PlusIcon />
				Add
			</Button>
			<Button variant="outline" size="sm" onclick={handleRemove} disabled={selectedIndex === null}>
				<MinusIcon />
				Remove
			</Button>
			<Button variant="ghost" size="sm" onclick={onClearAll} disabled={directories.length === 0}>
				Clear all
			</Button>
		</div>
	</div>

	<!-- Directory List / Drop Zone -->
	<div
		role="listbox"
		tabindex="0"
		aria-label="Selected directories"
		class={cn(
			'min-h-0 flex-1 overflow-y-auto rounded-md border-2 border-dashed bg-background p-2 transition-colors',
			isDragging
				? 'border-primary bg-primary/5'
				: 'border-input hover:border-muted-foreground/50',
			directories.length === 0 && 'flex items-center justify-center'
		)}
	>
		{#if directories.length === 0}
			<div class="flex flex-col items-center gap-2 text-muted-foreground">
				<FolderIcon class="size-6 opacity-50" />
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
</div>
