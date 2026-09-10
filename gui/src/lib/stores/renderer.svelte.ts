import { isTauri } from '@tauri-apps/api/core';
import type { CanvasRenderer, FolderColorMetadata, IconSizeSpec } from 'folco-renderer-wasm';
import {
	getFolderIconBase,
	getFolderIconSvg,
	getPlatformIconSizes
} from '$lib/services/tauri-commands';

export type RendererStatus = 'uninitialized' | 'loading' | 'ready' | 'error';

type WasmModule = typeof import('folco-renderer-wasm');

// SVG icons are resolution-independent, so the preview ladder is ours to pick.
const SVG_PREVIEW_SIZES = [16, 24, 32, 48, 64, 128, 256];

/** Logical (unscaled) sizes offered by the preview, deduped and ascending. */
function logicalSizes(items: { width: number; scale: number }[]): number[] {
	const sizes = items.map((item) => Math.round(item.width / item.scale));
	return [...new Set(sizes)].sort((a, b) => a - b);
}

// Shared WASM module singleton — loaded once, reused everywhere.
let wasmModule: WasmModule | null = null;
let wasmInitPromise: Promise<WasmModule> | null = null;

/**
 * Ensures the WASM module is loaded and initialized exactly once.
 * Safe to call from anywhere; concurrent calls share the same promise.
 */
export async function ensureWasm(): Promise<WasmModule> {
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

	/** Available logical icon sizes (pixels) offered by the preview. */
	availableSizes = $state<number[]>([]);

	/** Whether the active folder icon is vector (SVG) rather than raster. */
	isSvg = $state(false);

	/** Whether the base icon came from the user rather than the system. */
	isCustom = $state(false);

	/** Whether the active medium supports the decal layer. */
	supportsDecal = $state(true);

	/** Whether the active medium supports recoloring the whole icon. */
	supportsSolidColor = $state(true);

	/** All available folder color presets, populated once WASM is ready. */
	availableColors = $state<FolderColorMetadata[]>([]);

	/** Platform size ladder, fetched once — it can't change while we're running. */
	#platformSizes: IconSizeSpec[] | null = null;

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
	 * Vector platforms (e.g. GNOME) return SVG markup; everything else returns
	 * a PNG icon set. Either way the result is one `CanvasRenderer`.
	 */
	async init() {
		if (this.status === 'loading' || this.status === 'ready') return;

		this.status = 'loading';
		this.error = null;

		// The icon base comes from the native side. Tests can stand one up with
		// mockIPC() from @tauri-apps/api/mocks, which satisfies isTauri().
		if (!this.#hasBackend()) return;

		try {
			const [wasm, svgBase] = await Promise.all([ensureWasm(), getFolderIconSvg()]);

			this.availableColors = wasm.getAvailableColors();

			const { CanvasRenderer } = wasm;

			if (svgBase) {
				this.#adopt(CanvasRenderer.fromSvgFolderIconBase(svgBase), SVG_PREVIEW_SIZES);
			} else {
				const base = await getFolderIconBase();
				if (!base) {
					throw new Error('The backend reported no folder icon in either medium.');
				}
				this.#adopt(CanvasRenderer.fromFolderIconBase(base), logicalSizes(base.images));
			}
		} catch (e) {
			this.#fail(e, 'Failed to initialize renderer');
		}
	}

	/**
	 * Replaces the base with a user-supplied raster image.
	 *
	 * Accepts any encoded format the backend can decode (PNG, JPEG, WebP, ...).
	 */
	async loadCustomImage(imageData: Uint8Array) {
		await this.#loadCustom((wasm, specs) => wasm.CanvasRenderer.fromCustomImage(imageData, specs));
	}

	/** Replaces the base with user-supplied SVG markup, rasterized to platform sizes. */
	async loadCustomSvg(svg: string) {
		await this.#loadCustom((wasm, specs) => wasm.CanvasRenderer.fromCustomSvg(svg, specs));
	}

	/**
	 * Builds a custom-icon renderer against the platform's size ladder.
	 *
	 * Unlike {@linkcode init}, this is callable at any time — picking a new
	 * image after one is already loaded is the normal path.
	 */
	async #loadCustom(build: (wasm: WasmModule, specs: IconSizeSpec[]) => CanvasRenderer) {
		this.status = 'loading';
		this.error = null;

		if (!this.#hasBackend()) return;

		try {
			const [wasm, specs] = await Promise.all([ensureWasm(), this.#platformIconSizes()]);

			if (this.availableColors.length === 0) {
				this.availableColors = wasm.getAvailableColors();
			}

			this.#adopt(build(wasm, specs), logicalSizes(specs));
		} catch (e) {
			this.#fail(e, 'Failed to load custom image');
		}
	}

	async #platformIconSizes(): Promise<IconSizeSpec[]> {
		this.#platformSizes ??= await getPlatformIconSizes();
		return this.#platformSizes;
	}

	/** Records a missing backend as an error; returns false when unavailable. */
	#hasBackend() {
		if (isTauri()) return true;
		this.status = 'error';
		this.error = 'Tauri backend unavailable; the icon pipeline lives on the native side.';
		return false;
	}

	/** Installs a renderer and refreshes the capability flags it reports. */
	#adopt(renderer: CanvasRenderer, availableSizes: number[]) {
		this.renderer?.free();
		this.renderer = renderer;
		this.availableSizes = availableSizes;
		this.isSvg = renderer.isSvg();
		this.isCustom = renderer.isCustom();
		this.supportsDecal = renderer.supportsDecal();
		this.supportsSolidColor = renderer.supportsSolidColor();
		this.status = 'ready';
		this.version++;
	}

	#fail(e: unknown, context: string) {
		this.status = 'error';
		this.error = e instanceof Error ? e.message : String(e);
		console.error(`${context}:`, e);
	}

	/**
	 * Renders a preview of the current icon to an HTML canvas element.
	 */
	renderToCanvas(canvas: HTMLCanvasElement, size: number) {
		this.#assertRenderer();
		this.renderer!.renderToCanvas(canvas, size);
	}

	/**
	 * Renders the scalable SVG artifact that would be written to the system.
	 *
	 * Returns `null` on raster platforms, where the saved artifact is a PNG set.
	 */
	renderOutputSvg(): string | null {
		this.#assertRenderer();
		return this.renderer!.renderOutputSvg() ?? null;
	}

	/**
	 * Recolors the whole icon to a target RGB color.
	 * Ignored when `supportsSolidColor` is false.
	 */
	setSolidColor(targetR: number, targetG: number, targetB: number) {
		this.#assertRenderer();
		this.renderer!.setSolidColor(targetR, targetG, targetB);
		this.version++;
	}

	setSolidColorEnabled(enabled: boolean) {
		this.#assertRenderer();
		this.renderer!.setSolidColorEnabled(enabled);
		this.version++;
	}

	/**
	 * Sets the color-dot badge in the icon's bottom-right corner.
	 */
	setColorDot(r: number, g: number, b: number) {
		this.#assertRenderer();
		this.renderer!.setColorDot(r, g, b);
		this.version++;
	}

	setColorDotEnabled(enabled: boolean) {
		this.#assertRenderer();
		this.renderer!.setColorDotEnabled(enabled);
		this.version++;
	}

	/**
	 * Sets the decal configuration. Ignored when `supportsDecal` is false.
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
		this.renderer?.clearCache();
	}

	/**
	 * Frees the renderer and resets the store state.
	 */
	destroy() {
		this.renderer?.free();
		this.renderer = null;
		this.status = 'uninitialized';
		this.error = null;
		this.isCustom = false;
	}

	#assertRenderer() {
		if (this.status !== 'ready' || !this.renderer) {
			throw new Error('Renderer not initialized. Call init() first.');
		}
	}
}

export const renderer = new RendererStore();
