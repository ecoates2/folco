import type { IconCapabilities } from 'folco-renderer-wasm';

/**
 * What a customization starts from.
 *
 * This — not the medium — is what a workflow selects. Whether the base turns
 * out to be raster or vector is resolved afterwards and surfaced as
 * capabilities, so a new medium never needs a new workflow.
 */
export type WorkflowId = 'folder' | 'custom-image';

/** A control group the page knows how to render. */
export type SectionId = 'source-image' | 'solid-color' | 'color-dot' | 'emoji' | 'icon';

/** A layer, named as `IconCapabilities` names it. */
export type LayerName = keyof IconCapabilities;

export interface SectionSpec {
	id: SectionId;
	label: string;
	/**
	 * Layers this section drives. A section is available when the base can
	 * realize at least one of them. Omit for sections that configure no layer.
	 */
	layers?: LayerName[];
}

export interface WorkflowSpec {
	id: WorkflowId;
	title: string;
	/** Rendered in this order, each annotated with its availability. */
	sections: SectionSpec[];
}

/** A section paired with what the resolved base allows. */
export interface ResolvedSection extends SectionSpec {
	available: boolean;
	/** Why it's unavailable, straight from the renderer. */
	reason?: string;
}
