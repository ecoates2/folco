import { isTauri } from '@tauri-apps/api/core';
import type {
	CanvasRenderer,
	DefaultPlacements,
	FolderColorMetadata,
	IconCapabilities,
	IconSizeSpec,
	LayerRejections
} from 'folco-renderer-wasm';
import {
	getCustomImageBytes,
	getCustomImageSizes,
	getFolderIconBase,
	getFolderIconSvg,
	loadCustomImage,
	type CustomImage
} from '$lib/services/tauri-commands';

export type RendererStatus = 'uninitialized' | 'loading' | 'ready' | 'error';

type WasmModule = typeof import('folco-renderer-wasm');

// Optimistic default: before a base is resolved there is nothing to disable.
const ALL_CAPABILITIES: IconCapabilities = {
	solidColor: true,
	colorDot: true,
	decal: true,
	overlay: true
};

const NO_REJECTIONS: LayerRejections = {
	solidColor: undefined,
	colorDot: undefined,
	decal: undefined,
	overlay: undefined
};

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

	/** The backend's custom image this preview was built from, when `isCustom`. */
	customImage = $state<CustomImage | null>(null);

	/** Which layers the resolved base can realize. */
	capabilities = $state<IconCapabilities>(ALL_CAPABILITIES);

	/** Why each unrealizable layer is unavailable. */
	rejections = $state<LayerRejections>(NO_REJECTIONS);

	/** Where overlays and the color dot go on this base; shared with the backend. */
	defaultPlacements = $state<DefaultPlacements | null>(null);

	readonly supportsDecal = $derived(this.capabilities.decal);
	readonly supportsSolidColor = $derived(this.capabilities.solidColor);

	/** All available folder color presets, populated once WASM is ready. */
	availableColors = $state<FolderColorMetadata[]>([]);

	/** Custom-image size ladder, fetched once — it can't change while we're running. */
	#customSizes: IconSizeSpec[] | null = null;

	/**
	 * Monotonically increasing version counter, bumped on every customization
	 * state change. Components can track this in a `$effect` to trigger
	 * re-renders automatically.
	 */
	version = $state(0);

	/**
	 * Loads the system folder icon — the folder workflow's acquisition step.
	 *
	 * Vector platforms (e.g. GNOME) return SVG markup; everything else returns
	 * a PNG icon set. Either way the result is one `CanvasRenderer`. Callable
	 * repeatedly, so switching back from a custom image re-reads the base.
	 */
	async loadFolderIcon() {
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
				// Vector bases have no size ladder; the preview picks its own resolution.
				this.#adopt(CanvasRenderer.fromSvgFolderIconBase(svgBase), []);
			} else {
				const base = await getFolderIconBase();
				if (!base) {
					throw new Error('The backend reported no folder icon in either medium.');
				}
				this.#adopt(CanvasRenderer.fromFolderIconBase(base), logicalSizes(base.images));
			}
		} catch (e) {
			this.#fail(e, 'Failed to load the system folder icon');
		}
	}

	/**
	 * Replaces the base with a user-picked image file.
	 *
	 * The backend reads and keeps the file; the preview is built from the bytes it
	 * hands back, so it always shows what Apply will write.
	 */
	async loadCustomImageFile(path: string) {
		this.status = 'loading';
		this.error = null;

		if (!this.#hasBackend()) return;

		try {
			const [wasm, specs, image] = await Promise.all([
				ensureWasm(),
				this.#customImageSizes(),
				loadCustomImage(path)
			]);
			const bytes = await getCustomImageBytes(image.revision);

			if (this.availableColors.length === 0) {
				this.availableColors = wasm.getAvailableColors();
			}

			const { CanvasRenderer } = wasm;
			if (image.format === 'raster') {
				this.#adopt(CanvasRenderer.fromCustomImage(bytes, specs), logicalSizes(specs), image);
				return;
			}

			const svg = new TextDecoder().decode(bytes);
			if (image.medium === 'vector') {
				// Vector bases have no size ladder; the preview picks its own resolution.
				this.#adopt(CanvasRenderer.fromCustomSvg(svg), [], image);
			} else {
				this.#adopt(
					CanvasRenderer.fromCustomSvgRasterized(svg, specs),
					logicalSizes(specs),
					image
				);
			}
		} catch (e) {
			this.#fail(e, 'Failed to load custom image');
		}
	}

	async #customImageSizes(): Promise<IconSizeSpec[]> {
		this.#customSizes ??= await getCustomImageSizes();
		return this.#customSizes;
	}

	/** Records a missing backend as an error; returns false when unavailable. */
	#hasBackend() {
		if (isTauri()) return true;
		this.status = 'error';
		this.error = 'Tauri backend unavailable; the icon pipeline lives on the native side.';
		return false;
	}

	/** Installs a renderer and refreshes the capability flags it reports. */
	#adopt(renderer: CanvasRenderer, availableSizes: number[], customImage: CustomImage | null = null) {
		this.renderer?.free();
		this.renderer = renderer;
		this.customImage = customImage;
		this.availableSizes = availableSizes;
		this.isSvg = renderer.isSvg();
		this.isCustom = renderer.isCustom();
		this.capabilities = renderer.capabilities();
		this.rejections = renderer.layerRejections();
		this.defaultPlacements = renderer.defaultPlacements();
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

	/**
	 * Clears the solid color configuration, deactivating the layer.
	 */
	clearSolidColor() {
		this.#assertRenderer();
		this.renderer!.clearSolidColor();
		this.version++;
	}

	/**
	 * Sets the color-dot badge. Placement works exactly like `setOverlay`.
	 */
	setColorDot(
		r: number,
		g: number,
		b: number,
		position: string,
		anchorMode: string,
		scale: number
	) {
		this.#assertRenderer();
		this.renderer!.setColorDot(r, g, b, position, anchorMode, scale);
		this.version++;
	}

	/**
	 * Clears the color-dot configuration, deactivating the layer.
	 */
	clearColorDot() {
		this.#assertRenderer();
		this.renderer!.clearColorDot();
		this.version++;
	}

	/**
	 * Sets the decal configuration. Ignored when `supportsDecal` is false.
	 * Pass `null` to clear/deactivate.
	 */
	setDecal(svgData: string | null | undefined, scale: number) {
		this.#assertRenderer();
		this.renderer!.setDecal(svgData ?? null, scale);
		this.version++;
	}

	/**
	 * Sets the overlay configuration.
	 * Pass `null` to clear/deactivate.
	 */
	setOverlay(
		svgData: string | null | undefined,
		position: string,
		anchorMode: string,
		scale: number
	) {
		this.#assertRenderer();
		this.renderer!.setOverlay(svgData ?? null, position, anchorMode, scale);
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
		this.availableSizes = [];
		this.isSvg = false;
		this.isCustom = false;
		this.customImage = null;
		this.capabilities = ALL_CAPABILITIES;
		this.rejections = NO_REJECTIONS;
		this.defaultPlacements = null;
	}

	#assertRenderer() {
		if (this.status !== 'ready' || !this.renderer) {
			throw new Error('Renderer not initialized. Call init() first.');
		}
	}
}

export const renderer = new RendererStore();
