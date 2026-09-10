import { invoke } from '@tauri-apps/api/core';
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
