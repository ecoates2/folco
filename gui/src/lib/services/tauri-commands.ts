import { Channel, invoke } from '@tauri-apps/api/core';
import type {
	IconSizeSpec,
	SerializableFolderIconBase,
	SerializableSvgFolderIconBase
} from 'folco-renderer-wasm';

/** Wire shape of `folco_core::PlatformSizeSpec`. */
interface PlatformSizeSpec {
	sizes: IconSizeSpec[];
}

/**
 * The system folder icon as a PNG set.
 *
 * Returns `null` on platforms whose folder icon is vector — call
 * {@linkcode getFolderIconSvg} first and only fall back to this.
 */
export function getFolderIconBase(): Promise<SerializableFolderIconBase | null> {
	return invoke('get_folder_icon_base');
}

/**
 * The system folder icon as scalable markup.
 *
 * Returns `null` on platforms whose folder icon is raster.
 */
export function getFolderIconSvg(): Promise<SerializableSvgFolderIconBase | null> {
	return invoke('get_folder_icon_svg');
}

/**
 * The sizes a user-supplied image is rasterized to on the host platform.
 *
 * A user-supplied image has no inherent size ladder, so this is the only source
 * of truth for one. Platform knowledge lives in the backend; never hardcode
 * sizes here.
 */
export async function getCustomImageSizes(): Promise<IconSizeSpec[]> {
	const spec = await invoke<PlatformSizeSpec>('get_custom_image_sizes');
	return spec.sizes;
}

/** Whether `FOLCO_EXPERIMENTAL` was set when the app launched. */
export function isExperimentalEnabled(): Promise<boolean> {
	return invoke('experimental_enabled');
}

/** Wire shape of `folco_core::progress::Progress`. */
export type CustomizeProgress =
	| { kind: 'started'; total: number }
	| { kind: 'rendering' }
	| { kind: 'renderFailed'; error: string }
	| { kind: 'processing'; current: number; path: string }
	| { kind: 'folderComplete'; index: number; path: string }
	| { kind: 'folderFailed'; index: number; path: string; error: string }
	| { kind: 'completed'; succeeded: number; failed: number };

/**
 * Applies a `CustomizationProfile` JSON to the system folder icon and writes it
 * to each folder.
 *
 * Rejects only when the whole operation can't start (bad profile, unsupported
 * layers, context failure). Per-folder failures arrive as `folderFailed` events.
 */
export function customizeFolders(
	folders: string[],
	profileJson: string,
	onProgress: (event: CustomizeProgress) => void
): Promise<void> {
	const channel = new Channel<CustomizeProgress>(onProgress);
	return invoke('customize_folders', { folders, profileJson, onProgress: channel });
}

/** Restores the system default icon on each folder. Progress events as {@linkcode customizeFolders}. */
export function resetFolders(
	folders: string[],
	onProgress: (event: CustomizeProgress) => void
): Promise<void> {
	const channel = new Channel<CustomizeProgress>(onProgress);
	return invoke('reset_folders', { folders, onProgress: channel });
}

/** The custom image the backend holds. `revision` names this exact copy. */
export interface CustomImage {
	revision: number;
	/** How to decode the bytes. */
	format: 'svg' | 'raster';
	/** The pipeline the backend will write it through; the preview must match. */
	medium: 'vector' | 'raster';
	fileName: string;
}

/** Has the backend read `path` and hold it as the custom image, replacing any earlier one. */
export function loadCustomImage(path: string): Promise<CustomImage> {
	return invoke('load_custom_image', { path });
}

/** What custom images the host platform can write. Fixed for the app's lifetime. */
export interface CustomImageSupport {
	write: { raster: boolean; vector: boolean };
	/** Lowercase file extensions, without the dot. */
	extensions: string[];
}

export function getCustomImageSupport(): Promise<CustomImageSupport> {
	return invoke('get_custom_image_support');
}

/** The bytes of custom image `revision`; rejects if a newer image has replaced it. */
export async function getCustomImageBytes(revision: number): Promise<Uint8Array> {
	return new Uint8Array(await invoke<ArrayBuffer>('get_custom_image_bytes', { revision }));
}

/**
 * Uses custom image `imageRevision` as the folder icon, decorated per the
 * profile. Rejects if a newer image has replaced it.
 */
export function customizeFoldersWithImage(
	folders: string[],
	imageRevision: number,
	profileJson: string,
	onProgress: (event: CustomizeProgress) => void
): Promise<void> {
	const channel = new Channel<CustomizeProgress>(onProgress);
	return invoke('customize_folders_with_image', {
		folders,
		imageRevision,
		profileJson,
		onProgress: channel
	});
}
