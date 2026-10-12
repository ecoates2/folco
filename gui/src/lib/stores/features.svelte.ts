import { isExperimentalEnabled } from '$lib/services/tauri-commands';

class FeaturesStore {
	/** Gates UI for features that aren't ready to be on by default. */
	experimental = $state(false);

	async init() {
		try {
			this.experimental = await isExperimentalEnabled();
		} catch {
			// Not running under Tauri; experimental stays off.
		}
	}
}

export const features = new FeaturesStore();
