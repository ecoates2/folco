<script lang="ts">
  import "../app.css";
  import { theme } from "$lib/stores/theme.svelte";
  import { onMount } from "svelte";

  let { children } = $props();

  onMount(() => {
    theme.init();
    return () => theme.destroy();
  });
</script>
 
{@render children?.()}