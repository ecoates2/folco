<script lang="ts">
  import { FolderCustomization } from '$lib/components/app/folder-customization';
</script>

<main class="h-screen overflow-hidden">
  <FolderCustomization />
</main>

<style lang="postcss">
  @reference "tailwindcss";
  :global(html) {
    background-color: theme(--color-gray-100);
  }
</style>
