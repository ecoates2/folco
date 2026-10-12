import { open } from '@tauri-apps/plugin-dialog';

class DirectoryStore {
	paths = $state<string[]>([]);
	selectedIndex = $state<number | null>(null);

	/** Prompts the user for folders and adds the ones not already listed. */
	async browse() {
		const selected = await open({
			directory: true,
			multiple: true,
			title: 'Select Folder(s)'
		});
		if (!selected) return;
		this.add(Array.isArray(selected) ? selected : [selected]);
	}

	add(paths: string[]) {
		const fresh = paths.filter((p) => !this.paths.includes(p));
		this.paths = [...this.paths, ...fresh];
	}

	remove(index: number) {
		this.paths = this.paths.filter((_, i) => i !== index);
		this.selectedIndex = null;
	}

	clear() {
		this.paths = [];
		this.selectedIndex = null;
	}
}

export const directories = new DirectoryStore();
