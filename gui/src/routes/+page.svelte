<script lang="ts">
  import { FolderCustomization } from '$lib/components/app/folder-customization';
</script>

<main class="container mx-auto max-w-2xl p-6">
  <FolderCustomization />
</main>

<style lang="postcss">
  @reference "tailwindcss";
  :global(html) {
    background-color: theme(--color-gray-100);
  }
</style>
