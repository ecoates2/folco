import type { FolderColorMetadata } from 'folco-renderer-wasm';
import { renderer } from './renderer.svelte';

class SolidColorStore {
	enabled = $state(false);

	// Kept while toggled off so turning it back on restores the pick.
	#pick = $state<FolderColorMetadata | null>(null);

	/** The picked preset's id, or null if none was picked yet. */
	readonly selectedId = $derived(this.#pick?.id ?? null);

	setEnabled(on: boolean) {
		this.enabled = on;
		this.#push();
	}

	select(color: FolderColorMetadata) {
		this.#pick = color;
		this.enabled = true;
		this.#push();
	}

	/** Re-applies the solid color; call after the renderer is replaced. */
	reapply() {
		this.#push();
	}

	#push() {
		if (renderer.status !== 'ready') return;

		const pick = this.enabled ? this.#pick : null;
		if (pick) renderer.setSolidColor(pick.r, pick.g, pick.b);
		else renderer.clearSolidColor();
	}
}

export const solidColor = new SolidColorStore();
