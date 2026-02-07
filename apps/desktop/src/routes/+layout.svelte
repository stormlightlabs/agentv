<script>
  import { initLogger } from "$lib/stores/logger.svelte";
  import { onMount } from "svelte";
  import "../app.css";

  onMount(() => {
    initLogger();
  });
</script>

<slot />
