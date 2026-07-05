import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

export type ThemePreference = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';

const STORAGE_KEY = 'theme-preference';

class ThemeStore {
	preference = $state<ThemePreference>('system');
	resolved = $state<ResolvedTheme>('light');

	#unlistenThemeChange: (() => void) | null = null;

	constructor() {
		const stored = localStorage.getItem(STORAGE_KEY) as ThemePreference | null;
		if (stored && ['light', 'dark', 'system'].includes(stored)) {
			this.preference = stored;
		}
	}

	async init() {
		const appWindow = getCurrentWebviewWindow();

		// Get initial system theme
		const systemTheme = await appWindow.theme();
		this.#applyTheme(systemTheme ?? 'light');

		// Listen for system theme changes
		const unlisten = await appWindow.onThemeChanged(({ payload }) => {
			if (this.preference === 'system') {
				this.#applyTheme(payload);
			}
		});

		this.#unlistenThemeChange = unlisten;
	}

	setPreference(pref: ThemePreference) {
		this.preference = pref;
		localStorage.setItem(STORAGE_KEY, pref);

		if (pref === 'system') {
			// Re-read system theme
			getCurrentWebviewWindow()
				.theme()
				.then((systemTheme) => {
					this.#applyTheme(systemTheme ?? 'light');
				});
		} else {
			this.#applyTheme(pref);
		}
	}

	#applyTheme(theme: ResolvedTheme) {
		this.resolved = theme;
		document.documentElement.classList.toggle('dark', theme === 'dark');
	}

	destroy() {
		this.#unlistenThemeChange?.();
	}
}

export const theme = new ThemeStore();
