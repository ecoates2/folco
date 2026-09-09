import { invoke, isTauri } from '@tauri-apps/api/core';
import type {
	CanvasRenderer,
	FolderColorMetadata,
	SerializableFolderIconBase,
	SerializableSvgFolderIconBase
} from 'folco-renderer-wasm';

export type RendererStatus = 'uninitialized' | 'loading' | 'ready' | 'error';

// SVG icons are resolution-independent, so the preview ladder is ours to pick.
const SVG_PREVIEW_SIZES = [16, 24, 32, 48, 64, 128, 256];

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

	/** Available logical icon sizes (pixels) offered by the preview. */
	availableSizes = $state<number[]>([]);

	/** Whether the active folder icon is vector (SVG) rather than raster. */
	isSvg = $state(false);

	/** Whether the active medium supports the decal layer. */
	supportsDecal = $state(true);

	/** Whether the active medium supports recoloring the whole icon. */
	supportsSolidColor = $state(true);

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
	 * Vector platforms (e.g. GNOME) return SVG markup; everything else returns
	 * a PNG icon set. Either way the result is one `CanvasRenderer`.
	 */
	async init() {
		if (this.status === 'loading' || this.status === 'ready') return;

		this.status = 'loading';
		this.error = null;

		// The icon base comes from the native side. Tests can stand one up with
		// mockIPC() from @tauri-apps/api/mocks, which satisfies isTauri().
		if (!isTauri()) {
			this.status = 'error';
			this.error = 'Tauri backend unavailable; cannot load the folder icon base.';
			return;
		}

		try {
			const [wasm, svgBase] = await Promise.all([
				ensureWasm(),
				invoke<SerializableSvgFolderIconBase | null>('get_folder_icon_svg')
			]);

			this.availableColors = wasm.getAvailableColors();

			const { CanvasRenderer } = wasm;

			if (svgBase) {
				this.renderer = CanvasRenderer.fromSvgFolderIconBase(svgBase);
				this.availableSizes = SVG_PREVIEW_SIZES;
			} else {
				const base = await invoke<SerializableFolderIconBase>('get_folder_icon_base');
				this.renderer = CanvasRenderer.fromFolderIconBase(base);
				this.availableSizes = base.images
					.map((img) => Math.round(img.width / img.scale))
					.filter((size, i, arr) => arr.indexOf(size) === i)
					.sort((a, b) => a - b);
			}

			this.isSvg = this.renderer.isSvg();
			this.supportsDecal = this.renderer.supportsDecal();
			this.supportsSolidColor = this.renderer.supportsSolidColor();

			this.status = 'ready';
		} catch (e) {
			this.status = 'error';
			this.error = e instanceof Error ? e.message : String(e);
			console.error('Failed to initialize renderer:', e);
		}
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
	}

	#assertRenderer() {
		if (this.status !== 'ready' || !this.renderer) {
			throw new Error('Renderer not initialized. Call init() first.');
		}
	}
}

export const renderer = new RendererStore();
