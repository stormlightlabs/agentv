<script lang="ts">
  import type { EventData, SessionData } from "$lib/types";

  type Props = { session: SessionData; events: EventData[] };

  let { session, events }: Props = $props();

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function getEventIcon(kind: string): string {
    switch (kind) {
      case "message":
        return "i-ri-chat-1-line";
      case "tool_call":
        return "i-ri-tools-line";
      case "tool_result":
        return "i-ri-check-line";
      case "error":
        return "i-ri-error-warning-line";
      case "system":
        return "i-ri-settings-3-line";
      default:
        return "i-ri-circle-fill";
    }
  }

  function getRoleColor(role: string | null): string {
    switch (role) {
      case "user":
        return "var(--color-green-bright)";
      case "assistant":
        return "var(--color-blue-bright)";
      case "system":
        return "var(--color-fg-dim)";
      default:
        return "var(--color-fg-muted)";
    }
  }

  function getContentPreview(content: string | null): string {
    if (!content) return "";

    try {
      const parsed = JSON.parse(content);
      if (typeof parsed === "object") {
        if (parsed.name) return `[Tool: ${parsed.name}]`;
        if (parsed.content) return String(parsed.content).slice(0, 100);
        return JSON.stringify(parsed).slice(0, 100);
      }
    } catch {
      /* No-op */
    }
    return content.slice(0, 100).replace(/\n/g, " ");
  }

  function groupEventsByDate(events: EventData[]): Map<string, EventData[]> {
    const groups = new Map<string, EventData[]>();
    for (const event of events) {
      const date = new Date(event.timestamp).toLocaleDateString("en-US", {
        month: "long",
        day: "numeric",
        year: "numeric",
      });
      if (!groups.has(date)) {
        groups.set(date, []);
      }
      groups.get(date)!.push(event);
    }
    return groups;
  }

  let groupedEvents = $derived(groupEventsByDate(events));
</script>

<div class="session-viewer">
  <header class="session-header">
    <div class="header-main">
      <h2 class="session-title">{session.title || "Untitled Session"}</h2>
      <div class="session-meta">
        <span class="meta-item">
          <span class="meta-label">ID:</span>
          {session.external_id.slice(0, 8)}
        </span>
        <span class="meta-item">
          <span class="meta-label">Source:</span>
          {session.source}
        </span>
        {#if session.project}
          <span class="meta-item">
            <span class="meta-label">Project:</span>
            {session.project}
          </span>
        {/if}
        <span class="meta-item">
          <span class="meta-label">Events:</span>
          {events.length}
        </span>
      </div>
    </div>
    <div class="header-dates">
      <div class="date-item">
        <span class="date-label">Created:</span>
        {formatDate(session.created_at)}
      </div>
      <div class="date-item">
        <span class="date-label">Updated:</span>
        {formatDate(session.updated_at)}
      </div>
    </div>
  </header>

  <div class="events-container">
    {#if events.length === 0}
      <div class="empty-events">
        <p>No events in this session</p>
      </div>
    {:else}
      {#each groupedEvents.entries() as [date, dateEvents]}
        <div class="date-group">
          <div class="date-header">{date}</div>
          <div class="events-list">
            {#each dateEvents as event, index (event.id)}
              <div class="event-item">
                <div class="event-marker">
                  <span class="{getEventIcon(event.kind)} text-fg-dim"></span>
                </div>
                <div class="event-content">
                  <div class="event-header">
                    <span class="event-role" style="color: {getRoleColor(event.role)}">
                      {event.role || "unknown"}
                    </span>
                    <span class="event-kind">{event.kind}</span>
                    <span class="event-time">
                      {new Date(event.timestamp).toLocaleTimeString("en-US", {
                        hour: "2-digit",
                        minute: "2-digit",
                        second: "2-digit",
                      })}
                    </span>
                  </div>
                  {#if event.content}
                    <div class="event-text">
                      {getContentPreview(event.content)}
                    </div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .session-viewer {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .session-header {
    padding: 1rem 1.5rem;
    background-color: var(--color-bg-soft);
    border-bottom: 1px solid var(--color-bg-muted);
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
  }

  .header-main {
    flex: 1;
    min-width: 0;
  }

  .session-title {
    margin: 0 0 0.5rem 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--color-fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .session-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: var(--color-fg-dim);
  }

  .meta-item {
    display: flex;
    gap: 0.25rem;
  }

  .meta-label {
    color: var(--color-fg-muted);
  }

  .header-dates {
    text-align: right;
    font-size: 0.75rem;
    color: var(--color-fg-dim);
    flex-shrink: 0;
  }

  .date-item {
    margin-bottom: 0.25rem;
  }

  .date-label {
    color: var(--color-fg-muted);
  }

  .events-container {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 1.5rem;
  }

  .empty-events {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--color-fg-dim);
  }

  .date-group {
    margin-bottom: 1.5rem;
  }

  .date-header {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--color-fg-muted);
    margin-bottom: 0.75rem;
    padding-bottom: 0.25rem;
    border-bottom: 1px solid var(--color-bg-muted);
  }

  .events-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .event-item {
    display: flex;
    gap: 0.75rem;
    padding: 0.75rem;
    background-color: var(--color-bg-soft);
    border-radius: 4px;
    border: 1px solid transparent;
    transition: border-color 0.15s ease;
  }

  .event-item:hover {
    border-color: var(--color-bg-muted);
  }

  .event-marker {
    flex-shrink: 0;
    width: 24px;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 0.125rem;
  }

  .event-marker :global([class*="i-ri-"]) {
    font-size: 1rem;
    color: var(--color-fg-dim);
  }

  .event-content {
    flex: 1;
    min-width: 0;
  }

  .event-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.25rem;
    font-size: 0.75rem;
  }

  .event-role {
    font-weight: 600;
    text-transform: uppercase;
  }

  .event-kind {
    color: var(--color-fg-muted);
    text-transform: lowercase;
  }

  .event-time {
    color: var(--color-fg-dim);
    margin-left: auto;
  }

  .event-text {
    font-size: 0.8125rem;
    color: var(--color-fg);
    line-height: 1.5;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
  }
</style>
