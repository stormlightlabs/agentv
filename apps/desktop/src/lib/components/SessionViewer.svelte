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

<div class="flex-1 flex flex-col overflow-hidden">
  <header class="px-6 py-4 bg-bg-soft border-b border-bg-muted flex justify-between items-start gap-4">
    <div class="flex-1 min-w-0">
      <h2 class="m-0 mb-2 text-xl font-semibold text-fg overflow-hidden text-ellipsis whitespace-nowrap">
        {session.title || "Untitled Session"}
      </h2>
      <div class="flex flex-wrap gap-3 text-xs text-fg-dim">
        <span class="flex gap-1">
          <span class="text-fg-muted">ID:</span>
          {session.external_id.slice(0, 8)}
        </span>
        <span class="flex gap-1">
          <span class="text-fg-muted">Source:</span>
          {session.source}
        </span>
        {#if session.project}
          <span class="flex gap-1">
            <span class="text-fg-muted">Project:</span>
            {session.project}
          </span>
        {/if}
        <span class="flex gap-1">
          <span class="text-fg-muted">Events:</span>
          {events.length}
        </span>
      </div>
    </div>
    <div class="text-right text-xs text-fg-dim flex-shrink-0">
      <div class="mb-1">
        <span class="text-fg-muted">Created:</span>
        {formatDate(session.created_at)}
      </div>
      <div>
        <span class="text-fg-muted">Updated:</span>
        {formatDate(session.updated_at)}
      </div>
    </div>
  </header>

  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if events.length === 0}
      <div class="flex items-center justify-center h-full text-fg-dim">
        <p>No events in this session</p>
      </div>
    {:else}
      {#each groupedEvents.entries() as [date, dateEvents]}
        <div class="mb-6">
          <div class="text-xs font-semibold uppercase text-fg-muted mb-3 pb-1 border-b border-bg-muted">
            {date}
          </div>
          <div class="flex flex-col gap-2">
            {#each dateEvents as event, index (event.id)}
              <div class="flex gap-3 p-3 bg-bg-soft rounded border border-transparent transition-colors hover:border-bg-muted">
                <div class="flex-shrink-0 w-6 flex items-start justify-center pt-0.5">
                  <span class="{getEventIcon(event.kind)} text-fg-dim text-base"></span>
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-1 text-xs">
                    <span class="font-semibold uppercase" style="color: {getRoleColor(event.role)}">
                      {event.role || "unknown"}
                    </span>
                    <span class="text-fg-muted lowercase">{event.kind}</span>
                    <span class="text-fg-dim ml-auto">
                      {new Date(event.timestamp).toLocaleTimeString("en-US", {
                        hour: "2-digit",
                        minute: "2-digit",
                        second: "2-digit",
                      })}
                    </span>
                  </div>
                  {#if event.content}
                    <div class="text-sm text-fg leading-relaxed overflow-hidden text-ellipsis line-clamp-3">
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
