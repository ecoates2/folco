<script lang="ts">
  import "../app.css";
  import { theme } from "$lib/stores/theme.svelte";
  import { features } from "$lib/stores/features.svelte";
  import { platform } from "$lib/stores/platform.svelte";
  import { directories } from "$lib/stores/directories.svelte";
  import { errorDialog } from "$lib/stores/error-dialog.svelte";
  import { ErrorDialog } from "$lib/components/app/error-dialog";
  import { onMount } from "svelte";

  let { children } = $props();

  /**
   * Fades out the shell HTML's boot loader. The window is already visible and
   * painted in the right colour by this point, so this is purely the handoff
   * from the static loader to the mounted app.
   */
  function dismissBootLoader() {
    const loader = document.getElementById("boot-loader");
    if (!loader) return;

    loader.dataset.done = "";
    loader.addEventListener("transitionend", () => loader.remove(), { once: true });
    // Fallback for when transitions are disabled (e.g. reduced motion).
    setTimeout(() => loader.remove(), 400);
  }

  onMount(() => {
    theme.init();
    features.init();
    platform.init();
    directories.init();
    dismissBootLoader();

    return () => {
      theme.destroy();
    };
  });
</script>
 
{@render children?.()}

<ErrorDialog report={errorDialog.current} onDismiss={() => errorDialog.dismiss()} />