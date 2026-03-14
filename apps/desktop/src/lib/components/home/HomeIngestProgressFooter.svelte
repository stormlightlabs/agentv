<script lang="ts">
  import type { IngestProgress } from "$lib/types";
  import { fade } from "svelte/transition";

  type Props = {
    ingestProgress: IngestProgress | null;
  };

  let { ingestProgress }: Props = $props();

  function getSourceLabel(source: string): string {
    const trimmed = source.trim();
    if (!trimmed || trimmed === "[object PointerEvent]") return "sessions";
    return trimmed;
  }
</script>

{#if ingestProgress}
  {@const phase = ingestProgress.phase || "Ingesting"}
  {@const source = getSourceLabel(ingestProgress.source)}
  {@const percent = ingestProgress.total > 0 ? Math.round((ingestProgress.current / ingestProgress.total) * 100) : 0}
  <div class="fixed right-0 bottom-0 left-0 z-50" transition:fade={{ duration: 200 }}>
    <div
      class="bg-surface-soft/90 border-surface-muted text-fg-dim flex items-center justify-between border-t px-3 py-1 text-xs backdrop-blur-sm">
      <span>
        {phase} {source}... {ingestProgress.current}/{ingestProgress.total}
      </span>
      <span>{percent}%</span>
    </div>
    <div class="bg-surface-muted h-1">
      <div
        class="bg-blue h-full transition-all duration-300 ease-out"
        style="width: {percent}%">
      </div>
    </div>
  </div>
{/if}
