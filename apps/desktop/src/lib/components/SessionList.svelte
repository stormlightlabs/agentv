<script lang="ts">
  import type { SessionData } from "$lib/types";

  type Props = {
    sessions: SessionData[];
    selectedSession: SessionData | null;
    onSelect: (session: SessionData) => void;
  };

  let { sessions, selectedSession, onSelect }: Props = $props();

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }

  function getSessionTitle(session: SessionData): string {
    return session.title || session.external_id.slice(0, 8) || "Untitled";
  }

  function getProjectName(session: SessionData): string {
    if (!session.project) return "No project";
    const parts = session.project.split("-");
    return parts[parts.length - 1] || session.project;
  }
</script>

<div class="flex-1 flex flex-col overflow-hidden">
  <div class="px-4 py-2 border-b border-bg-muted flex justify-between items-center">
    <span class="text-xs text-fg-muted">{sessions.length} sessions</span>
  </div>

  <div class="flex-1 overflow-y-auto p-2">
    {#each sessions as session (session.id)}
      <button
        class="w-full p-3 mb-1 bg-transparent border border-transparent rounded text-left cursor-pointer transition-all font-inherit hover:bg-bg-muted"
        class:bg-bg-muted={selectedSession?.id === session.id}
        class:border-blue={selectedSession?.id === session.id}
        onclick={() => onSelect(session)}>
        <div class="flex justify-between items-center mb-1">
          <span class="font-medium text-fg text-sm overflow-hidden text-ellipsis whitespace-nowrap flex-1 mr-2">
            {getSessionTitle(session)}
          </span>
          <span class="text-2xs uppercase px-1.5 py-0.5 bg-blue text-bg rounded flex-shrink-0">
            {session.source}
          </span>
        </div>
        <div class="flex justify-between items-center text-xs text-fg-dim">
          <span class="overflow-hidden text-ellipsis whitespace-nowrap flex-1 mr-2">
            {getProjectName(session)}
          </span>
          <span class="shrink-0">{formatDate(session.updated_at)}</span>
        </div>
      </button>
    {/each}
  </div>
</div>
