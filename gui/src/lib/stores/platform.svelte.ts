import { getCustomImageSupport, type CustomImageSupport } from '$lib/services/tauri-commands';

class PlatformStore {
	/** What custom images this system can write; `null` until loaded or without a backend. */
	customImageSupport = $state<CustomImageSupport | null>(null);

	async init() {
		try {
			this.customImageSupport = await getCustomImageSupport();
		} catch {
			// Not running under Tauri; custom images can't be picked anyway.
		}
	}
}

export const platform = new PlatformStore();
