<script lang="ts">
  import { useToast } from "$lib/stores/toast.svelte";
  import type { ContentBlock, EventData, EventPayload, ExportFormat, SessionData } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { fade } from "svelte/transition";
  import CostLatencyPanel from "./CostLatencyPanel.svelte";

  type Props = {
    session: SessionData;
    events: EventData[];
    onSelectEvent?: (event: EventData) => void;
    onOpenDrawer?: () => void;
  };

  let { session, events, onSelectEvent, onOpenDrawer }: Props = $props();

  const toast = useToast();
  let exporting = $state(false);

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

  function extractToolCalls(rawPayload: EventPayload | null | undefined): Array<{ name: string; id: string }> {
    if (!rawPayload) return [];
    const message = rawPayload.message;
    if (!message) return [];

    const content = message.content;
    if (!Array.isArray(content)) return [];

    return content
      .filter(
        (block): block is ContentBlock & { type: "tool_use"; name: string; id: string } =>
          block.type === "tool_use" && typeof (block as { name?: string }).name === "string",
      )
      .map((block) => ({ name: block.name, id: block.id }));
  }

  function extractThinking(rawPayload: EventPayload | null | undefined): string | null {
    if (!rawPayload) return null;
    const message = rawPayload.message;
    if (!message) return null;

    const content = message.content;
    if (!Array.isArray(content)) return null;

    const thinkingBlock = content.find(
      (block): block is ContentBlock & { type: "thinking"; thinking: string } =>
        block.type === "thinking" && typeof (block as { thinking?: string }).thinking === "string",
    );
    return thinkingBlock?.thinking ?? null;
  }

  const extractGitBranch = (rawPayload?: EventPayload | null): string | null => rawPayload?.gitBranch ?? null;

  const extractCwd = (rawPayload?: EventPayload | null): string | null => rawPayload?.cwd ?? null;

  function extractContentFromPayload(payload: EventPayload | null | undefined): string | null {
    if (!payload) return null;

    const messageContent = payload.message?.content;
    if (messageContent) {
      if (typeof messageContent === "string") {
        return messageContent;
      }
      if (Array.isArray(messageContent)) {
        return messageContent
          .filter(
            (block): block is ContentBlock & { type: "text"; text: string } => block.type === "text" && "text" in block,
          )
          .map((block) => block.text)
          .join("\n");
      }
    }

    if (payload.content && typeof payload.content === "string") {
      return payload.content;
    }

    return null;
  }

  function getContentPreview(event: EventData): string {
    if (event.content) {
      return event.content.slice(0, 150).replace(/\n/g, " ");
    }

    const payloadContent = extractContentFromPayload(event.raw_payload);
    if (payloadContent) {
      return payloadContent.slice(0, 150).replace(/\n/g, " ");
    }

    const toolCalls = extractToolCalls(event.raw_payload);
    if (toolCalls.length > 0) {
      return `Tool calls: ${toolCalls.map((t) => t.name).join(", ")}`;
    }

    const thinking = extractThinking(event.raw_payload);
    if (thinking) {
      return `Thinking: ${thinking.slice(0, 100)}...`;
    }

    return "";
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

  async function exportSession(format: ExportFormat) {
    exporting = true;
    try {
      const content = await invoke<string>("export_session", { sessionId: session.id, format });

      const blob = new Blob([content], { type: format === "md" ? "text/markdown" : "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `session-${session.external_id.slice(0, 8)}.${format}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      toast.success(`Exported session as ${format.toUpperCase()}`);
    } catch (e) {
      toast.error(`Failed to export: ${e}`);
    } finally {
      exporting = false;
    }
  }

  let groupedEvents = $derived(groupEventsByDate(events));
  let expandedEvents = $state<Set<string>>(new Set());

  function toggleEvent(eventId: string) {
    const newSet = new Set(expandedEvents);
    if (newSet.has(eventId)) {
      newSet.delete(eventId);
    } else {
      newSet.add(eventId);
    }
    expandedEvents = newSet;
  }
</script>

<div class="flex-1 flex flex-col overflow-hidden">
  <header class="px-6 py-4 bg-surface-soft border-b border-surface-muted flex justify-between items-start gap-4">
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
    <div class="text-right text-xs text-fg-dim shrink-0">
      <div class="mb-1">
        <span class="text-fg-muted">Created:</span>
        {formatDate(session.created_at)}
      </div>
      <div class="mb-2">
        <span class="text-fg-muted">Updated:</span>
        {formatDate(session.updated_at)}
      </div>
      <div class="flex gap-1 justify-end">
        {#if onOpenDrawer}
          <button
            class="px-2 py-1 bg-surface border border-surface-muted rounded text-xs cursor-pointer transition-colors hover:border-blue hover:text-blue"
            onclick={onOpenDrawer}
            title="View session details">
            <span class="i-ri-information-line"></span>
            Details
          </button>
        {/if}
        <button
          class="px-2 py-1 bg-surface border border-surface-muted rounded text-xs cursor-pointer transition-colors hover:border-blue hover:text-blue disabled:opacity-50"
          onclick={() => exportSession("md")}
          disabled={exporting}
          title="Export as Markdown">
          .md
        </button>
        <button
          class="px-2 py-1 bg-surface border border-surface-muted rounded text-xs cursor-pointer transition-colors hover:border-blue hover:text-blue disabled:opacity-50"
          onclick={() => exportSession("json")}
          disabled={exporting}
          title="Export as JSON">
          .json
        </button>
        <button
          class="px-2 py-1 bg-surface border border-surface-muted rounded text-xs cursor-pointer transition-colors hover:border-blue hover:text-blue disabled:opacity-50"
          onclick={() => exportSession("jsonl")}
          disabled={exporting}
          title="Export as JSONL">
          .jsonl
        </button>
      </div>
    </div>
  </header>

  <CostLatencyPanel {session} />

  <div class="flex-1 overflow-y-auto px-6 py-4">
    {#if events.length === 0}
      <div class="flex items-center justify-center h-full text-fg-dim" transition:fade>
        <p>No events in this session</p>
      </div>
    {:else}
      {#each groupedEvents.entries() as [date, dateEvents]}
        <div class="mb-6">
          <div class="text-xs font-semibold uppercase text-fg-muted mb-3 pb-1 border-b border-surface-muted">
            {date}
          </div>
          <div class="flex flex-col gap-2">
            {#each dateEvents as event (event.id)}
              {@const toolCalls = extractToolCalls(event.raw_payload)}
              {@const thinking = extractThinking(event.raw_payload)}
              {@const gitBranch = extractGitBranch(event.raw_payload)}
              {@const cwd = extractCwd(event.raw_payload)}
              {@const isExpanded = expandedEvents.has(event.id)}

              <div
                class="flex gap-3 p-3 bg-surface-soft rounded border border-transparent transition-colors hover:border-surface-muted cursor-pointer group"
                onclick={() => toggleEvent(event.id)}
                ondblclick={() => onSelectEvent?.(event)}
                onkeydown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    toggleEvent(event.id);
                  } else if (e.key === "i") {
                    onSelectEvent?.(event);
                  }
                }}
                role="button"
                tabindex="0"
                aria-expanded={isExpanded}>
                <div class="shrink-0 w-6 flex items-start justify-center pt-0.5">
                  <span class="{getEventIcon(event.kind)} text-fg-dim text-base"></span>
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-1 text-xs">
                    <span class="font-semibold uppercase" style="color: {getRoleColor(event.role)}">
                      {event.role || "unknown"}
                    </span>
                    <span class="text-fg-muted lowercase">{event.kind}</span>
                    {#if gitBranch}
                      <span class="text-fg-dim">
                        <span class="i-ri-git-branch-line"></span>
                        {gitBranch}
                      </span>
                    {/if}
                    <span class="text-fg-dim ml-auto">
                      {new Date(event.timestamp).toLocaleTimeString("en-US", {
                        hour: "2-digit",
                        minute: "2-digit",
                        second: "2-digit",
                      })}
                    </span>
                  </div>

                  {#if cwd && isExpanded}
                    <div class="text-xs text-fg-muted mb-2 font-mono">
                      {cwd}
                    </div>
                  {/if}

                  {#if event.content}
                    <div class="text-sm text-fg leading-relaxed overflow-hidden text-ellipsis line-clamp-3">
                      {getContentPreview(event)}
                    </div>
                  {/if}

                  {#if toolCalls.length > 0}
                    <div class="mt-2 flex flex-wrap gap-1">
                      {#each toolCalls as tool}
                        <span
                          class="inline-flex items-center gap-1 px-2 py-0.5 bg-surface-muted rounded text-xs text-fg-dim">
                          <span class="i-ri-tools-line"></span>
                          {tool.name}
                        </span>
                      {/each}
                    </div>
                  {/if}

                  {#if thinking && isExpanded}
                    <div class="mt-3 p-3 bg-surface-muted rounded border-l-2 border-blue-500">
                      <div class="text-xs font-semibold text-fg-muted mb-2 flex items-center gap-1">
                        <span class="i-ri-brain-line"></span>
                        Thinking
                      </div>
                      <div class="text-sm text-fg-dim whitespace-pre-wrap font-mono">
                        {thinking.slice(0, 500)}{thinking.length > 500 ? "..." : ""}
                      </div>
                    </div>
                  {/if}

                  {#if isExpanded && event.raw_payload && Object.keys(event.raw_payload).length > 0}
                    <div class="mt-3 p-2 bg-surface-muted rounded">
                      <div class="flex items-center justify-between mb-1">
                        <div class="text-xs font-semibold text-fg-muted">Raw Data</div>
                        <button
                          class="text-xs text-blue hover:text-blue-bright flex items-center gap-1 z-10"
                          onclick={(e) => {
                            e.stopPropagation();
                            onSelectEvent?.(event);
                          }}
                          type="button">
                          <span class="i-ri-eye-line"></span>
                          Inspect
                        </button>
                      </div>
                      <pre class="text-xs text-fg-dim overflow-x-auto"><code
                          >{JSON.stringify(event.raw_payload, null, 2).slice(0, 1000)}</code></pre>
                    </div>
                  {/if}
                </div>
                <button
                  class="opacity-0 group-hover:opacity-100 p-1 text-fg-dim hover:text-blue transition-opacity z-10"
                  title="Inspect event (I)"
                  onclick={(e) => {
                    e.stopPropagation();
                    onSelectEvent?.(event);
                  }}
                  type="button">
                  <span class="i-ri-eye-line"></span>
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>
