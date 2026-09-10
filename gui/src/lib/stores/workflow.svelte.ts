import { WORKFLOWS } from '$lib/workflows/definitions';
import type { ResolvedSection, WorkflowId, WorkflowSpec } from '$lib/workflows/types';
import { artwork } from './artwork.svelte';
import { renderer } from './renderer.svelte';

/** Pairs each declared section with what the resolved base actually allows. */
function resolveSections(spec: WorkflowSpec): ResolvedSection[] {
	const { capabilities, rejections } = renderer;

	return spec.sections.map((section) => {
		if (!section.layers) return { ...section, available: true };

		// One usable layer is enough: the icon section still works as an overlay
		// on a base that can't take decals.
		const available = section.layers.some((layer) => capabilities[layer]);
		if (available) return { ...section, available };

		return {
			...section,
			available,
			reason: section.layers.map((layer) => rejections[layer]).find(Boolean)
		};
	});
}

class WorkflowStore {
	id = $state<WorkflowId>('folder');

	readonly spec = $derived(WORKFLOWS[this.id]);

	/** Sections to render, in order, annotated with availability. */
	readonly sections = $derived.by(() => resolveSections(this.spec));

	/**
	 * Switches workflows and runs that workflow's acquisition step.
	 *
	 * Folder mode can fetch its base immediately; custom mode has nothing to
	 * load until the user supplies an image, so it only changes the UI.
	 */
	async activate(id: WorkflowId) {
		this.id = id;
		if (id === 'folder') await this.#load(() => renderer.loadFolderIcon());
	}

	/** Adopts a user-supplied raster image as the base. */
	async useImage(imageData: Uint8Array) {
		this.id = 'custom-image';
		await this.#load(() => renderer.loadCustomImage(imageData));
	}

	/** Adopts user-supplied SVG markup as the base. */
	async useSvg(svg: string) {
		this.id = 'custom-image';
		await this.#load(() => renderer.loadCustomSvg(svg));
	}

	// Swapping the renderer resets its layers, so decorations are pushed again.
	async #load(acquire: () => Promise<void>) {
		await acquire();
		artwork.reapply();
	}
}

export const workflow = new WorkflowStore();
