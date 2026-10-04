import type { Emoji } from '@emoji-mart/data';
import type { EmojiPickerSkin, SelectedEmoji } from '$lib/components/ui/emoji-picker/types';
import { renderer } from './renderer.svelte';

/** A piece of artwork placed on the icon, tagged by where it came from. */
export type Artwork =
	| { kind: 'emoji'; emoji: string; data: Emoji; skin: number }
	| { kind: 'icon'; svg: string };

/** The active artwork source, or `'none'`. */
export type ArtworkKind = Artwork['kind'] | 'none';

/** Which layer the artwork is drawn into. */
export type Placement = 'decal' | 'overlay';

/** How an overlay attaches to its anchor point. */
export type AnchorMode = 'inset' | 'centered';

const OVERLAY_POSITION = 'bottom-right';
const OVERLAY_SCALE = 0.5;
const DECAL_SCALE = 0.7;

class ArtworkStore {
	/**
	 * The active artwork source.
	 *
	 * Emoji and icons compete for a single overlay layer, so this is one value
	 * rather than a flag per source — the exclusion is structural.
	 */
	kind = $state<ArtworkKind>('none');

	/** Requested placement. Ignored when the artwork or medium can't honour it. */
	placement = $state<Placement>('decal');

	anchorMode = $state<AnchorMode>('inset');

	// Kept per source so toggling one off and back on restores the pick.
	// Emoji data is set once in selectEmoji and never mutated; skin is stored
	// separately so tone changes are a single primitive write — no spread, no
	// shared mutable references.
	#emoji = $state<{ kind: 'emoji'; data: Emoji } | null>(null);
	#skin = $state<number | null>(null);
	#icon = $state<Extract<Artwork, { kind: 'icon' }> | null>(null);

	/** Whether an emoji was selected. */
	#isEmoji = $derived(this.kind === 'emoji' && this.#emoji !== null && this.#skin !== null);

	/** Whether an icon was selected. */
	#isIcon = $derived(this.kind === 'icon');

	/** The currently selected emoji, or null. */
	#activeEmoji = $derived(
		this.#isEmoji
			? { kind: 'emoji' as const, data: this.#emoji!.data, skin: this.#skin! }
			: null
	);

	/** The currently selected icon, or null. */
	#activeIcon = $derived(this.#isIcon ? this.#icon : null);

	/** The native emoji character for the active skin tone. */
	#activeEmojiNative = $derived(
		this.#activeEmoji !== null
			? this.#activeEmoji.data.skins[this.#activeEmoji.skin].native
			: null
	);

	/** The active artwork, composed at read time so emoji is always up to date. */
	readonly selection = $derived<Artwork | null>(
		this.#isEmoji
			? { kind: 'emoji', emoji: this.#activeEmojiNative!, data: this.#activeEmoji!.data, skin: this.#activeEmoji!.skin }
			: this.#isIcon
				? this.#activeIcon
				: null
	);

	/**
	 * Where the artwork actually lands.
	 *
	 * Decals are monochrome imprints tinted against the folder, so emoji can
	 * only be overlays; vector folder icons have no decal layer at all.
	 */
	readonly effectivePlacement = $derived<Placement>(
		this.selection?.kind === 'icon' && this.placement === 'decal' && renderer.supportsDecal
			? 'decal'
			: 'overlay'
	);

	/** Switches the active source, or clears it with `'none'`. */
	setKind(kind: ArtworkKind) {
		this.kind = kind;
		this.#push();
	}

	selectEmoji(selected: SelectedEmoji) {
		this.#emoji = { kind: 'emoji', data: selected.data };
		this.#skin = selected.skin;
		this.kind = 'emoji';
		this.#push();
	}

	/** Switches to a new skin tone for the current emoji. */
	setEmojiSkin(skin: EmojiPickerSkin) {
		if (this.#emoji === null || this.#emoji.data.skins.length <= 1) return;
		this.#skin = skin;
		this.#push();
	}

	selectIcon(svg: string) {
		this.#icon = { kind: 'icon', svg };
		this.kind = 'icon';
		this.#push();
	}

	setPlacement(placement: Placement) {
		this.placement = placement;
		this.#push();
	}

	setAnchorMode(anchorMode: AnchorMode) {
		this.anchorMode = anchorMode;
		this.#push();
	}

	/** Re-applies the current selection; call after the renderer is replaced. */
	reapply() {
		this.#push();
	}

	/** Writes the current selection into the renderer's decal and overlay layers. */
	#push() {
		if (renderer.status !== 'ready') return;

		const art = this.selection;
		const placement = this.effectivePlacement;

		// Decal and overlay are separate layers, so both are written every time:
		// setting only the one that gained the artwork would leave the other
		// still configured, and still rendering.
		renderer.setDecal(art?.kind === 'icon' && placement === 'decal' ? art.svg : null, DECAL_SCALE);

		if (!art || placement !== 'overlay') {
			renderer.setOverlay(null, OVERLAY_POSITION, this.anchorMode, OVERLAY_SCALE);
		} else if (art.kind === 'emoji') {
			renderer.setOverlayEmoji(art.emoji, OVERLAY_POSITION, this.anchorMode, OVERLAY_SCALE);
		} else {
			renderer.setOverlay(art.svg, OVERLAY_POSITION, this.anchorMode, OVERLAY_SCALE);
		}
	}
}

export const artwork = new ArtworkStore();
