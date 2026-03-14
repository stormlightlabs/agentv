<script lang="ts">
  import Modal from "$lib/components/Modal.svelte";
  import { useToast } from "$lib/stores/toast.svelte";
  import type { SessionData } from "$lib/types";
  import { getDisplayExternalId, getDisplayProject, getDisplaySessionTitle } from "$lib/utils/sessionDisplay";

  type Props = {
    open: boolean;
    selectedSession: SessionData | null;
    eventsCount: number;
    onOpenChange?: (open: boolean) => void;
  };

  let { open, selectedSession, eventsCount, onOpenChange }: Props = $props();

  const toast = useToast();

  async function copySessionJson() {
    if (!selectedSession) return;

    try {
      await navigator.clipboard.writeText(JSON.stringify(selectedSession, null, 2));
      toast.success("Copied to clipboard");
    } catch {
      toast.error("Failed to copy");
    }
  }

  const displayTitle = $derived(selectedSession ? getDisplaySessionTitle(selectedSession) : "Untitled Session");
  const displayProject = $derived(selectedSession ? getDisplayProject(selectedSession.project) : "No project");
  const displayExternalId = $derived(
    selectedSession ? getDisplayExternalId(selectedSession.source, selectedSession.external_id) : "unknown",
  );
</script>

{#if selectedSession}
  <Modal {open} {onOpenChange} size="xl" contentClass="h-[85vh] flex flex-col" aria-label="Session details">
    <div class="border-surface-muted bg-surface-soft flex items-center justify-between gap-2 border-b px-6 py-4">
      <div class="flex items-center gap-3">
        <h2 class="text-fg m-0 text-xl font-semibold">
          {displayTitle}
        </h2>
        <span class="bg-surface-muted text-fg-dim rounded px-2 py-0.5 text-xs uppercase">
          {selectedSession.source}
        </span>
      </div>
      <div class="flex items-center gap-2">
        <button
          class="bg-surface border-surface-muted text-fg hover:border-blue hover:text-blue flex items-center gap-1 rounded border px-2 py-1.5 text-sm transition-colors"
          onclick={copySessionJson}
          type="button">
          <span class="i-ri-file-copy-line"></span>
          <span class="sr-only">Copy Session JSON</span>
        </button>
        <button
          class="text-fg-dim hover:text-fg p-2 transition-colors"
          onclick={() => onOpenChange?.(false)}
          type="button"
          aria-label="Close session details">
          <span class="i-ri-close-line text-xl"></span>
        </button>
      </div>
    </div>
    <div class="flex-1 overflow-auto p-6">
      <div class="mb-6 grid grid-cols-3 gap-4 text-sm">
        <div class="bg-surface-soft border-surface-muted rounded border p-3">
          <div class="text-fg-muted mb-1 text-xs">Session ID</div>
          <div class="text-fg font-mono text-xs">{selectedSession.id}</div>
        </div>
        <div class="bg-surface-soft border-surface-muted rounded border p-3">
          <div class="text-fg-muted mb-1 text-xs">External ID</div>
          <div class="text-fg font-mono text-xs">{displayExternalId}</div>
        </div>
        <div class="bg-surface-soft border-surface-muted rounded border p-3">
          <div class="text-fg-muted mb-1 text-xs">Project</div>
          <div class="text-fg">{displayProject}</div>
        </div>
        <div class="bg-surface-soft border-surface-muted rounded border p-3">
          <div class="text-fg-muted mb-1 text-xs">Created</div>
          <div class="text-fg">{new Date(selectedSession.created_at).toLocaleString()}</div>
        </div>
        <div class="bg-surface-soft border-surface-muted rounded border p-3">
          <div class="text-fg-muted mb-1 text-xs">Updated</div>
          <div class="text-fg">{new Date(selectedSession.updated_at).toLocaleString()}</div>
        </div>
        <div class="bg-surface-soft border-surface-muted rounded border p-3">
          <div class="text-fg-muted mb-1 text-xs">Events</div>
          <div class="text-fg">{eventsCount} events</div>
        </div>
      </div>
      <div class="bg-surface-soft border-surface-muted overflow-hidden rounded border">
        <div class="border-surface-muted bg-surface-muted/50 flex items-center justify-between border-b px-4 py-2">
          <span class="text-fg text-sm font-semibold">Full Session Data</span>
          <span class="text-fg-dim text-xs">JSON</span>
        </div>
        <pre class="text-fg-dim max-h-[50vh] overflow-x-auto p-4 text-sm"><code
            >{JSON.stringify(selectedSession, null, 2)}</code></pre>
      </div>
    </div>
  </Modal>
{/if}
