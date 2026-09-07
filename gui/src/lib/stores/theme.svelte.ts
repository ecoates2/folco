import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

export type ThemePreference = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';

const STORAGE_KEY = 'theme-preference';
const DARK_QUERY = '(prefers-color-scheme: dark)';

class ThemeStore {
	preference = $state<ThemePreference>('system');
	resolved = $state<ResolvedTheme>('light');

	#systemTheme: ResolvedTheme = 'light';
	#mq: MediaQueryList | null = null;
	#onSystemChange: ((e: MediaQueryListEvent) => void) | null = null;
	#unlistenTauri: (() => void) | null = null;

	constructor() {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (stored === 'light' || stored === 'dark' || stored === 'system') {
			this.preference = stored;
		}
	}

	async init() {
		// Linux caveat: Tauri's GTK3/WebKitGTK webview resolves prefers-color-scheme from the
		// `gtk-theme` setting, not GNOME's `color-scheme` toggle. Under a pinned dark GTK3 theme
		// (e.g. adw-gtk3-dark) it reports dark permanently and neither listener below ever fires.
		// A real fix means reading org.freedesktop.appearance color-scheme from the XDG portal.
		this.#mq = window.matchMedia(DARK_QUERY);
		this.#systemTheme = this.#mq.matches ? 'dark' : 'light';
		this.#onSystemChange = (e) => this.#setSystemTheme(e.matches ? 'dark' : 'light');
		this.#mq.addEventListener('change', this.#onSystemChange);
		this.#apply();

		// Tauri reports OS theme changes the webview's media query can miss.
		try {
			this.#unlistenTauri = await getCurrentWebviewWindow().onThemeChanged(({ payload }) =>
				this.#setSystemTheme(payload)
			);
		} catch {
			// Not running under Tauri; the media query listener covers it.
		}
	}

	setPreference(pref: ThemePreference) {
		this.preference = pref;
		localStorage.setItem(STORAGE_KEY, pref);
		this.#apply();
	}

	#setSystemTheme(theme: ResolvedTheme) {
		this.#systemTheme = theme;
		if (this.preference === 'system') {
			this.#apply();
		}
	}

	#apply() {
		const theme = this.preference === 'system' ? this.#systemTheme : this.preference;
		this.resolved = theme;
		document.documentElement.classList.toggle('dark', theme === 'dark');

		// Cache for the next launch so the window can be created with a matching
		// background colour instead of flashing the wrong one.
		invoke('set_startup_theme', { theme }).catch(() => {
			// Not running under Tauri; nothing to cache.
		});
	}

	destroy() {
		if (this.#mq && this.#onSystemChange) {
			this.#mq.removeEventListener('change', this.#onSystemChange);
		}
		this.#unlistenTauri?.();
		this.#mq = null;
		this.#onSystemChange = null;
		this.#unlistenTauri = null;
	}
}

export const theme = new ThemeStore();
