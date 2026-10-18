import type { FolderColorMetadata } from 'folco-renderer-wasm';
import { renderer } from './renderer.svelte';

class SolidColorStore {
	// Kept on bases that can't take a solid color so switching back restores it.
	#pick = $state<FolderColorMetadata | null>(null);

	/** The picked preset's id, or null for no solid color. */
	readonly selectedId = $derived(this.#pick?.id ?? null);

	/** Picks a preset, or clears the solid color with `null`. */
	select(color: FolderColorMetadata | null) {
		this.#pick = color;
		this.#push();
	}

	/** Re-applies the solid color; call after the renderer is replaced. */
	reapply() {
		this.#push();
	}

	#push() {
		if (renderer.status !== 'ready') return;

		const pick = renderer.supportsSolidColor ? this.#pick : null;
		if (pick) renderer.setSolidColor(pick.r, pick.g, pick.b);
		else renderer.clearSolidColor();
	}
}

export const solidColor = new SolidColorStore();
