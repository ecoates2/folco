export interface FolderFailure {
	path: string;
	error: string;
}

export interface ErrorReport {
	title: string;
	message: string;
	/** Per-folder errors, shown as a table below the message. */
	failures?: FolderFailure[];
}

function messageOf(e: unknown): string {
	return e instanceof Error ? e.message : String(e);
}

class ErrorDialogStore {
	#queue = $state<ErrorReport[]>([]);

	/** The report on screen; later ones wait until it's dismissed. */
	readonly current = $derived(this.#queue[0] ?? null);

	show(report: ErrorReport) {
		this.#queue.push(report);
	}

	/** Shows a single caught error. */
	showError(title: string, e: unknown) {
		this.show({ title, message: messageOf(e) });
	}

	dismiss() {
		this.#queue.shift();
	}
}

export const errorDialog = new ErrorDialogStore();
