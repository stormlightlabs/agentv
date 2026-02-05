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

<div class="session-list">
  <div class="list-header">
    <span class="count">{sessions.length} sessions</span>
  </div>

  <div class="list-content">
    {#each sessions as session (session.id)}
      <button
        class="session-item"
        class:selected={selectedSession?.id === session.id}
        onclick={() => onSelect(session)}>
        <div class="session-header">
          <span class="session-title">{getSessionTitle(session)}</span>
          <span class="session-source">{session.source}</span>
        </div>
        <div class="session-meta">
          <span class="project-name">{getProjectName(session)}</span>
          <span class="session-date">{formatDate(session.updated_at)}</span>
        </div>
      </button>
    {/each}
  </div>
</div>

<style>
  .session-list {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .list-header {
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--color-bg-muted);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .count {
    font-size: 0.75rem;
    color: var(--color-fg-muted);
  }

  .list-content {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .session-item {
    width: 100%;
    padding: 0.75rem;
    margin-bottom: 0.25rem;
    background-color: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    text-align: left;
    cursor: pointer;
    transition: all 0.15s ease;
    font-family: inherit;
  }

  .session-item:hover {
    background-color: var(--color-bg-muted);
  }

  .session-item.selected {
    background-color: var(--color-bg-muted);
    border-color: var(--color-blue);
  }

  .session-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.25rem;
  }

  .session-title {
    font-weight: 500;
    color: var(--color-fg);
    font-size: 0.875rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    margin-right: 0.5rem;
  }

  .session-source {
    font-size: 0.625rem;
    text-transform: uppercase;
    padding: 0.125rem 0.375rem;
    background-color: var(--color-blue);
    color: var(--color-bg);
    border-radius: 3px;
    flex-shrink: 0;
  }

  .session-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
    color: var(--color-fg-dim);
  }

  .project-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    margin-right: 0.5rem;
  }

  .session-date {
    flex-shrink: 0;
  }
</style>
