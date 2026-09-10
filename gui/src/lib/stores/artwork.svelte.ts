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
	#emoji = $state<Extract<Artwork, { kind: 'emoji' }> | null>(null);
	#icon = $state<Extract<Artwork, { kind: 'icon' }> | null>(null);

	readonly selection = $derived<Artwork | null>(
		this.kind === 'emoji' ? this.#emoji : this.kind === 'icon' ? this.#icon : null
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
		this.#emoji = {
			kind: 'emoji',
			emoji: selected.emoji,
			data: selected.data,
			skin: selected.skin
		};
		this.kind = 'emoji';
		this.#push();
	}

	/** Re-emits the current emoji at a new skin tone. */
	setEmojiSkin(skin: EmojiPickerSkin) {
		const current = this.#emoji;
		if (!current || current.data.skins.length <= 1) return;
		this.#emoji = { ...current, emoji: current.data.skins[skin].native, skin };
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
