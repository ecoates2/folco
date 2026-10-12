import { customizeFolders, type CustomizeProgress } from '$lib/services/tauri-commands';
import { directories } from './directories.svelte';
import { renderer } from './renderer.svelte';

export interface FolderFailure {
	path: string;
	error: string;
}

export type ApplyStatus = 'idle' | 'running' | 'done';

class ApplyStore {
	status = $state<ApplyStatus>('idle');
	total = $state(0);
	processed = $state(0);
	succeeded = $state(0);

	/** Error that stopped the whole run (bad profile, unsupported layers, render failure). */
	error = $state<string | null>(null);

	/** Folders that failed individually while the rest went through. */
	failures = $state<FolderFailure[]>([]);

	/** Why Apply can't run right now, or `null` when it can. */
	readonly blockedReason = $derived.by(() => {
		if (this.status === 'running') return 'Already applying';
		if (renderer.status !== 'ready') return 'The icon has not loaded yet';
		if (renderer.isCustom) return 'Applying custom images is not supported yet';
		if (directories.paths.length === 0) return 'Add at least one folder';
		return null;
	});

	/** Exports the renderer's profile and writes it to every listed folder. */
	async run() {
		if (this.blockedReason) return;

		const folders = $state.snapshot(directories.paths);
		this.status = 'running';
		this.total = folders.length;
		this.processed = 0;
		this.succeeded = 0;
		this.error = null;
		this.failures = [];

		try {
			await customizeFolders(folders, renderer.exportProfileJson(), (event) =>
				this.#onProgress(event)
			);
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
			this.status = 'done';
		}
	}

	// `completed` (not the invoke resolving) ends the run: channel messages can
	// land after the command's response.
	#onProgress(event: CustomizeProgress) {
		switch (event.kind) {
			case 'started':
				this.total = event.total;
				break;
			case 'renderFailed':
				this.error = event.error;
				break;
			case 'folderComplete':
				this.processed++;
				break;
			case 'folderFailed':
				this.processed++;
				this.failures.push({ path: event.path, error: event.error });
				break;
			case 'completed':
				this.succeeded = event.succeeded;
				this.status = 'done';
				break;
		}
	}
}

export const apply = new ApplyStore();
