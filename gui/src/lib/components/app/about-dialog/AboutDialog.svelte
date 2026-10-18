<script lang="ts">
	import { getVersion } from '@tauri-apps/api/app';
	import { resolveResource } from '@tauri-apps/api/path';
	import { readTextFile } from '@tauri-apps/plugin-fs';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import * as Dialog from '$lib/components/ui/dialog';
	import { getAppMetadata, type AppMetadata } from '$lib/services/tauri-commands';
	import { onMount } from 'svelte';

	interface Props {
		open?: boolean;
	}

	let { open = $bindable(false) }: Props = $props();

	let version = $state<string | null>(null);
	let metadata = $state<AppMetadata | null>(null);
	let iconUrl = $state<string | null>(null);

	/**
	 * Tauri only exposes the window icon as a fixed-size raster, which blurs when
	 * scaled, so the SVG source is bundled as a resource instead.
	 */
	async function loadIconUrl(): Promise<string> {
		const svg = await readTextFile(await resolveResource('app-icon.svg'));
		return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
	}

	/** Splits Cargo's `Name <email>` author form. */
	function parseAuthor(author: string): { name: string; email: string | null } {
		const match = /^(.*?)\s*<([^>]+)>\s*$/.exec(author);
		return match ? { name: match[1], email: match[2] } : { name: author, email: null };
	}

	function openExternal(event: MouseEvent, url: string): void {
		event.preventDefault();
		openUrl(url).catch(() => {});
	}

	onMount(() => {
		// These reject outside Tauri (browser dev server); missing pieces are just omitted.
		const ignore = () => {};
		getVersion().then((v) => (version = v), ignore);
		getAppMetadata().then((m) => (metadata = m), ignore);
		loadIconUrl().then((url) => (iconUrl = url), ignore);
	});
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-sm">
		<Dialog.Header class="items-center text-center sm:text-center">
			{#if iconUrl}
				<img src={iconUrl} alt="" class="mb-2 size-20" draggable="false" />
			{/if}
			<Dialog.Title class="text-2xl">Folco</Dialog.Title>
			{#if version}
				<p class="flex items-center gap-2 text-sm text-muted-foreground">
					Version {version}
					{#if metadata?.debug}
						<span
							class="rounded-sm bg-amber-500/15 px-1.5 py-0.5 text-xs font-medium text-amber-700 dark:text-amber-400"
						>
							Debug build
						</span>
					{/if}
				</p>
			{/if}
			<Dialog.Description class="pt-2">
				{metadata?.description ?? ''}
			</Dialog.Description>
		</Dialog.Header>

		{#if metadata}
			<dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
				{#if metadata.authors.length > 0}
					<dt class="text-muted-foreground">
						{metadata.authors.length === 1 ? 'Author' : 'Authors'}
					</dt>
					<dd>
						<ul>
							{#each metadata.authors.map(parseAuthor) as author (author.name)}
								<li>
									{author.name}
									{#if author.email}
										<a
											href="mailto:{author.email}"
											class="block text-muted-foreground underline-offset-4 hover:underline"
											onclick={(e) => openExternal(e, `mailto:${author.email}`)}
										>
											{author.email}
										</a>
									{/if}
								</li>
							{/each}
						</ul>
					</dd>
				{/if}
				{#if metadata.repository}
					<dt class="text-muted-foreground">Source</dt>
					<dd>
						<a
							href={metadata.repository}
							class="break-all text-primary underline-offset-4 hover:underline"
							onclick={(e) => openExternal(e, metadata!.repository)}
						>
							{metadata.repository.replace(/^https?:\/\//, '')}
						</a>
					</dd>
				{/if}
			</dl>
		{/if}
	</Dialog.Content>
</Dialog.Root>
