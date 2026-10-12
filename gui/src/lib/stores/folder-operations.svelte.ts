import {
	customizeFolders,
	resetFolders,
	type CustomizeProgress
} from '$lib/services/tauri-commands';
import { artwork } from './artwork.svelte';
import { directories } from './directories.svelte';
import { errorDialog, type FolderFailure } from './error-dialog.svelte';
import { renderer } from './renderer.svelte';
import { solidColor } from './solid-color.svelte';

export type FolderOperation = 'apply' | 'reset';
export type OperationStatus = 'idle' | 'running' | 'done';

type ProgressHandler = (event: CustomizeProgress) => void;

const FAILURE_TITLES: Record<FolderOperation, { whole: string; partial: string }> = {
	apply: {
		whole: "Couldn't apply customization",
		partial: "Some folders couldn't be customized"
	},
	reset: { whole: "Couldn't reset folders", partial: "Some folders couldn't be reset" }
};

class FolderOperationsStore {
	/** The most recently started operation. */
	operation = $state<FolderOperation>('apply');
	status = $state<OperationStatus>('idle');
	total = $state(0);
	processed = $state(0);
	succeeded = $state(0);

	/** Error that stopped the whole run (bad profile, unsupported layers, render failure). */
	error = $state<string | null>(null);

	/** Folders that failed individually while the rest went through. */
	failures = $state<FolderFailure[]>([]);

	/** Whether any option the current icon can realize is switched on and picked. */
	readonly hasCustomization = $derived(
		(solidColor.enabled && solidColor.selectedId !== null && renderer.supportsSolidColor) ||
			artwork.selection !== null
	);

	/** Why Reset can't run right now, or `null` when it can. */
	readonly resetBlockedReason = $derived.by(() => {
		if (this.status === 'running') return 'Another operation is running';
		if (directories.paths.length === 0) return 'Add at least one folder';
		return null;
	});

	/** Why Apply can't run right now, or `null` when it can. */
	readonly applyBlockedReason = $derived.by(() => {
		if (this.resetBlockedReason) return this.resetBlockedReason;
		if (renderer.status !== 'ready') return 'The icon has not loaded yet';
		if (renderer.isCustom) return 'Applying custom images is not supported yet';
		if (!this.hasCustomization) return 'Enable at least one customization option';
		return null;
	});

	/** Exports the renderer's profile and writes it to every listed folder. */
	apply() {
		if (this.applyBlockedReason) return;
		return this.#run('apply', (folders, onProgress) =>
			customizeFolders(folders, renderer.exportProfileJson(), onProgress)
		);
	}

	/** Restores the system default icon on every listed folder. */
	reset() {
		if (this.resetBlockedReason) return;
		return this.#run('reset', resetFolders);
	}

	async #run(
		operation: FolderOperation,
		start: (folders: string[], onProgress: ProgressHandler) => Promise<void>
	) {
		const folders = $state.snapshot(directories.paths);
		this.operation = operation;
		this.status = 'running';
		this.total = folders.length;
		this.processed = 0;
		this.succeeded = 0;
		this.error = null;
		this.failures = [];

		try {
			await start(folders, (event) => this.#onProgress(event));
		} catch (e) {
			this.error = e instanceof Error ? e.message : String(e);
			this.#finish();
		}
	}

	#finish() {
		this.status = 'done';
		const titles = FAILURE_TITLES[this.operation];

		if (this.error) {
			errorDialog.show({ title: titles.whole, message: this.error });
		} else if (this.failures.length) {
			errorDialog.show({
				title: titles.partial,
				message: `${this.failures.length} of ${this.total} folders failed.`,
				failures: $state.snapshot(this.failures)
			});
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
				this.#finish();
				break;
		}
	}
}

export const folderOperations = new FolderOperationsStore();
