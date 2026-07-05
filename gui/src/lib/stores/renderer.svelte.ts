import { invoke } from '@tauri-apps/api/core';
import type { CanvasRenderer, FolderColorMetadata, SerializableFolderIconBase } from 'folco-renderer-wasm';

export type RendererStatus = 'uninitialized' | 'loading' | 'ready' | 'error';

// Shared WASM module singleton — loaded once, reused everywhere.
let wasmModule: typeof import('folco-renderer-wasm') | null = null;
let wasmInitPromise: Promise<typeof import('folco-renderer-wasm')> | null = null;

/**
 * Ensures the WASM module is loaded and initialized exactly once.
 * Safe to call from anywhere; concurrent calls share the same promise.
 */
export async function ensureWasm(): Promise<typeof import('folco-renderer-wasm')> {
	if (wasmModule) return wasmModule;
	if (!wasmInitPromise) {
		wasmInitPromise = import('folco-renderer-wasm').then(async (mod) => {
			await mod.default();
			wasmModule = mod;
			return mod;
		});
	}
	return wasmInitPromise;
}

class RendererStore {
	status = $state<RendererStatus>('uninitialized');
	error = $state<string | null>(null);
	renderer = $state<CanvasRenderer | null>(null);

	/** Available logical icon sizes (pixels), derived from the base icon set. */
	availableSizes = $state<number[]>([]);

	/** All available folder color presets, populated once WASM is ready. */
	availableColors = $state<FolderColorMetadata[]>([]);

	/**
	 * Monotonically increasing version counter, bumped on every customization
	 * state change. Components can track this in a `$effect` to trigger
	 * re-renders automatically.
	 */
	version = $state(0);

	/**
	 * Initializes the WASM module and creates a `CanvasRenderer` from the
	 * backend's `CustomizationContext` icon base.
	 *
	 * This performs two steps:
	 * 1. Loads the folco-renderer-wasm module
	 * 2. Fetches the icon base from the Tauri backend via IPC
	 * 3. Creates a `CanvasRenderer` from the received data
	 */
	async init() {
		if (this.status === 'loading' || this.status === 'ready') return;

		this.status = 'loading';
		this.error = null;

		try {
			// Load WASM module and fetch icon base in parallel
			const [wasm, folderIconBase] = await Promise.all([
				ensureWasm(),
				invoke<SerializableFolderIconBase>('get_folder_icon_base')
			]);

			// Populate available colors from WASM
			this.availableColors = wasm.getAvailableColors();

			// Compute available logical sizes from the icon images
			this.availableSizes = folderIconBase.images
				.map((img) => Math.round(img.width / img.scale))
				.filter((size, i, arr) => arr.indexOf(size) === i)
				.sort((a, b) => a - b);

			const { CanvasRenderer } = wasm;
			this.renderer = CanvasRenderer.fromFolderIconBase(folderIconBase);

			this.status = 'ready';
		} catch (e) {
			this.status = 'error';
			this.error = e instanceof Error ? e.message : String(e);
			console.error('Failed to initialize renderer:', e);
		}
	}

	/**
	 * Renders the current icon to an HTML canvas element.
	 */
	renderToCanvas(canvas: HTMLCanvasElement, size: number) {
		this.#assertRenderer();
		this.renderer!.renderToCanvas(canvas, size);
	}

	/**
	 * Renders the current icon and returns raw RGBA pixel data.
	 */
	renderToPixels(size: number): Uint8Array {
		this.#assertRenderer();
		return this.renderer!.renderToPixels(size);
	}

	/**
	 * Returns the dimensions of the rendered icon at the given logical size.
	 */
	getRenderedDimensions(size: number): [number, number] {
		this.#assertRenderer();
		return this.renderer!.getRenderedDimensions(size) as [number, number];
	}

	/**
	 * Sets the color target from a target RGB color.
	 */
	setFolderColorTarget(targetR: number, targetG: number, targetB: number) {
		this.#assertRenderer();
		this.renderer!.setFolderColorTarget(targetR, targetG, targetB);
		this.version++;
	}

	setFolderColorTargetEnabled(enabled: boolean) {
		this.#assertRenderer();
		this.renderer!.setFolderColorTargetEnabled(enabled);
		this.version++;
	}

	/**
	 * Sets the decal configuration.
	 */
	setDecal(svgData: string | null | undefined, scale: number) {
		this.#assertRenderer();
		this.renderer!.setDecal(svgData, scale);
		this.version++;
	}

	setDecalEnabled(enabled: boolean) {
		this.#assertRenderer();
		this.renderer!.setDecalEnabled(enabled);
		this.version++;
	}

	/**
	 * Sets the overlay configuration.
	 */
	setOverlay(
		svgData: string | null | undefined,
		position: string,
		anchorMode: string,
		scale: number
	) {
		this.#assertRenderer();
		this.renderer!.setOverlay(svgData, position, anchorMode, scale);
		this.version++;
	}

	/**
	 * Sets the overlay to an emoji character.
	 */
	setOverlayEmoji(emoji: string, position: string, anchorMode: string, scale: number) {
		this.#assertRenderer();
		try {
			this.renderer!.setOverlayEmoji(emoji, position, anchorMode, scale);
		} catch (e) {
			console.error('Failed to set overlay emoji:', e);
		}
		this.version++;
	}

	setOverlayEnabled(enabled: boolean) {
		this.#assertRenderer();
		this.renderer!.setOverlayEnabled(enabled);
		this.version++;
	}

	/**
	 * Exports the current settings as a JSON string.
	 */
	exportProfileJson(): string {
		this.#assertRenderer();
		return this.renderer!.exportProfileJson();
	}

	/**
	 * Imports settings from a JSON string.
	 */
	importProfileJson(json: string) {
		this.#assertRenderer();
		this.renderer!.importProfileJson(json);
		this.version++;
	}

	/**
	 * Clears all customizations and returns to the base icon.
	 */
	reset() {
		this.#assertRenderer();
		this.renderer!.reset();
		this.version++;
	}

	/**
	 * Clears the render cache to free memory.
	 */
	clearCache() {
		this.#assertRenderer();
		this.renderer!.clearCache();
	}

	/**
	 * Frees the renderer and resets the store state.
	 */
	destroy() {
		this.renderer?.free();
		this.renderer = null;
		this.status = 'uninitialized';
		this.error = null;
	}

	#assertReady() {
		if (this.status !== 'ready') {
			throw new Error('Renderer not initialized. Call init() first.');
		}
	}

	#assertRenderer() {
		if (this.status !== 'ready' || !this.renderer) {
			throw new Error('Renderer not initialized. Call init() first.');
		}
	}
}

export const renderer = new RendererStore();
