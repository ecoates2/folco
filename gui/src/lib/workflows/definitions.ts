import type { WorkflowId, WorkflowSpec } from './types';

/**
 * The customization workflows, keyed by where the base comes from.
 *
 * These declare *structure* only — which sections appear and in what order.
 * How a section maps to renderer calls stays in the components, because those
 * mappings have interdependencies that a schema would have to grow escape
 * hatches for.
 */
export const WORKFLOWS: Record<WorkflowId, WorkflowSpec> = {
	folder: {
		id: 'folder',
		title: 'Customize Folder',
		sections: [
			{ id: 'solid-color', label: 'Solid Color', layers: ['solidColor'] },
			{ id: 'color-dot', label: 'Color Dot', layers: ['colorDot'] },
			{ id: 'emoji', label: 'Emoji Overlay', layers: ['overlay'] },
			{ id: 'icon', label: 'Icon', layers: ['decal', 'overlay'] }
		]
	},
	'custom-image': {
		id: 'custom-image',
		title: 'Custom Icon',
		sections: [
			// No layers: this section supplies the base rather than decorating it.
			{ id: 'source-image', label: 'Source Image' },
			{ id: 'color-dot', label: 'Color Dot', layers: ['colorDot'] },
			{ id: 'emoji', label: 'Emoji Overlay', layers: ['overlay'] },
			{ id: 'icon', label: 'Icon', layers: ['decal', 'overlay'] }
		]
	}
};
