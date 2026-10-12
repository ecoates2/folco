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
 * The icon sizes the host platform expects.
 *
 * A user-supplied image has no inherent size ladder, so this is the only source
 * of truth for one. Platform knowledge lives in the backend; never hardcode
 * sizes here.
 */
export async function getPlatformIconSizes(): Promise<IconSizeSpec[]> {
	const spec = await invoke<PlatformSizeSpec>('get_platform_icon_sizes');
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
