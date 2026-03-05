<script lang="ts">
  import { useToast } from "$lib/stores/toast.svelte";
  import type { ContentBlock, EventData, EventPayload, ExportFormat, SessionData } from "$lib/types";
  import {
    getDisplayExternalId,
    getDisplayProject,
    getDisplaySessionTitle,
    getSessionSlug,
  } from "$lib/utils/sessionDisplay";
  import { invoke } from "@tauri-apps/api/core";
  import { fade } from "svelte/transition";
  import CostLatencyPanel from "./CostLatencyPanel.svelte";
  import LiveLogViewer from "./LiveLogViewer.svelte";

  type Props = {
    session: SessionData;
    events: EventData[];
    onSelectEvent?: (event: EventData) => void;
    onOpenDrawer?: () => void;
  };

  let { session, events, onSelectEvent, onOpenDrawer }: Props = $props();

  const toast = useToast();
  let exporting = $state(false);
  let viewMode = $state<"static" | "live">("static");
  const displayTitle = $derived(getDisplaySessionTitle(session));
  const displayProject = $derived(getDisplayProject(session.project));
  const displayExternalId = $derived(getDisplayExternalId(session.source, session.external_id));

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
      case "message": {
        return "i-ri-chat-1-line";
      }
      case "tool_call": {
        return "i-ri-tools-line";
      }
      case "tool_result": {
        return "i-ri-check-line";
      }
      case "error": {
        return "i-ri-error-warning-line";
      }
      case "system": {
        return "i-ri-settings-3-line";
      }
      default: {
        return "i-ri-circle-fill";
      }
    }
  }

  function getRoleColor(role: string | null): string {
    switch (role) {
      case "user": {
        return "var(--color-green-bright)";
      }
      case "assistant": {
        return "var(--color-blue-bright)";
      }
      case "system": {
        return "var(--color-fg-dim)";
      }
      default: {
        return "var(--color-fg-muted)";
      }
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
      return event.content.slice(0, 150).replaceAll("\n", " ");
    }

    const payloadContent = extractContentFromPayload(event.raw_payload);
    if (payloadContent) {
      return payloadContent.slice(0, 150).replaceAll("\n", " ");
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

  type GroupedEvents = { date: string; events: EventData[] };

  function groupEventsByDate(events: EventData[]): GroupedEvents[] {
    const groups: Record<string, EventData[]> = {};
    for (const event of events) {
      const date = new Date(event.timestamp).toLocaleDateString("en-US", {
        month: "long",
        day: "numeric",
        year: "numeric",
      });
      if (!groups[date]) groups[date] = [];
      groups[date].push(event);
    }
    return Object.entries(groups).map(([date, dateEvents]) => ({ date, events: dateEvents }));
  }

  async function exportSession(format: ExportFormat) {
    exporting = true;
    try {
      const content = await invoke<string>("export_session", { sessionId: session.id, format });

      const blob = new Blob([content], { type: format === "md" ? "text/markdown" : "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `session-${getSessionSlug(session.source, session.external_id)}.${format}`;
      document.body.append(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);

      toast.success(`Exported session as ${format.toUpperCase()}`);
    } catch (error) {
      toast.error(`Failed to export: ${error}`);
    } finally {
      exporting = false;
    }
  }

  let groupedEvents = $derived(groupEventsByDate(events));
  let expandedEvents = $state<Record<string, boolean>>({});

  function toggleEvent(eventId: string) {
    expandedEvents = { ...expandedEvents, [eventId]: !expandedEvents[eventId] };
  }
</script>

<div class="flex min-w-0 flex-1 flex-col overflow-hidden">
  <header
    class="bg-surface-soft border-surface-muted sticky top-0 z-20 flex flex-wrap items-start justify-between gap-4 border-b px-6 py-4">
    <div class="min-w-0 flex-1">
      <h2 class="text-fg m-0 mb-2 overflow-hidden text-xl font-semibold text-ellipsis whitespace-nowrap">
        {displayTitle}
      </h2>
      <div class="text-fg-dim flex flex-wrap gap-3 text-xs">
        <span class="flex gap-1">
          <span class="text-fg-muted">ID:</span>
          {displayExternalId}
        </span>
        <span class="flex gap-1">
          <span class="text-fg-muted">Source:</span>
          {session.source}
        </span>
        {#if session.project}
          <span class="flex gap-1">
            <span class="text-fg-muted">Project:</span>
            {displayProject}
          </span>
        {/if}
        <span class="flex gap-1">
          <span class="text-fg-muted">Events:</span>
          {events.length}
        </span>
      </div>
    </div>
    <div class="text-fg-dim max-w-full min-w-0 text-right text-xs">
      <div class="mb-1">
        <span class="text-fg-muted">Created:</span>
        {formatDate(session.created_at)}
      </div>
      <div class="mb-2">
        <span class="text-fg-muted">Updated:</span>
        {formatDate(session.updated_at)}
      </div>
      <div class="flex flex-wrap justify-end gap-1">
        <button
          class="cursor-pointer rounded border px-2 py-1 text-xs transition-colors {viewMode === 'live'
            ? 'bg-green/10 border-green text-green'
            : 'bg-surface border-surface-muted hover:border-green hover:text-green'}"
          onclick={() => (viewMode = viewMode === "live" ? "static" : "live")}
          title="Toggle live log view">
          <span class="i-ri-live-line"></span>
          Live
        </button>
        {#if onOpenDrawer}
          <button
            class="bg-surface border-surface-muted hover:border-blue hover:text-blue cursor-pointer rounded border px-2 py-1 text-xs transition-colors"
            onclick={onOpenDrawer}
            title="View session details">
            <span class="i-ri-information-line"></span>
            Details
          </button>
        {/if}
        <button
          class="bg-surface border-surface-muted hover:border-blue hover:text-blue cursor-pointer rounded border px-2 py-1 text-xs transition-colors disabled:opacity-50"
          onclick={() => exportSession("md")}
          disabled={exporting}
          title="Export as Markdown">
          .md
        </button>
        <button
          class="bg-surface border-surface-muted hover:border-blue hover:text-blue cursor-pointer rounded border px-2 py-1 text-xs transition-colors disabled:opacity-50"
          onclick={() => exportSession("json")}
          disabled={exporting}
          title="Export as JSON">
          .json
        </button>
        <button
          class="bg-surface border-surface-muted hover:border-blue hover:text-blue cursor-pointer rounded border px-2 py-1 text-xs transition-colors disabled:opacity-50"
          onclick={() => exportSession("jsonl")}
          disabled={exporting}
          title="Export as JSONL">
          .jsonl
        </button>
      </div>
    </div>
  </header>

  <CostLatencyPanel {session} />

  {#if viewMode === "live"}
    <div class="flex-1 overflow-hidden">
      <LiveLogViewer sessionId={session.external_id} initialEvents={events} />
    </div>
  {:else}
    <div class="flex-1 overflow-y-auto px-6 py-4">
      {#if events.length === 0}
        <div class="text-fg-dim flex h-full items-center justify-center" transition:fade>
          <p>No events in this session</p>
        </div>
      {:else}
        {#each groupedEvents as group (group.date)}
          <div class="mb-6">
            <div class="text-fg-muted border-surface-muted mb-3 border-b pb-1 text-xs font-semibold uppercase">
              {group.date}
            </div>
            <div class="flex flex-col gap-2">
              {#each group.events as event (event.id)}
                {@const toolCalls = extractToolCalls(event.raw_payload)}
                {@const thinking = extractThinking(event.raw_payload)}
                {@const gitBranch = extractGitBranch(event.raw_payload)}
                {@const cwd = extractCwd(event.raw_payload)}
                {@const isExpanded = Boolean(expandedEvents[event.id])}

                <div
                  class="bg-surface-soft hover:border-surface-muted group flex cursor-pointer gap-3 rounded border border-transparent p-3 transition-colors"
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
                  <div class="flex w-6 shrink-0 items-start justify-center pt-0.5">
                    <span class="{getEventIcon(event.role ? event.kind : 'system')} text-fg-dim text-base"></span>
                  </div>
                  <div class="min-w-0 flex-1">
                    <div class="mb-1 flex items-center gap-2 text-xs">
                      <span class="font-semibold uppercase" style="color: {getRoleColor(event.role)}">
                        {event.role || "system"}
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
                      <div class="text-fg-muted mb-2 font-mono text-xs">
                        {cwd}
                      </div>
                    {/if}

                    {#if event.content}
                      <div class="text-fg line-clamp-3 overflow-hidden text-sm leading-relaxed text-ellipsis">
                        {getContentPreview(event)}
                      </div>
                    {/if}

                    {#if toolCalls.length > 0}
                      <div class="mt-2 flex flex-wrap gap-1">
                        {#each toolCalls as tool, index (tool.id || `${tool.name}-${index}`)}
                          <span
                            class="bg-surface-muted text-fg-dim inline-flex items-center gap-1 rounded px-2 py-0.5 text-xs">
                            <span class="i-ri-tools-line"></span>
                            {tool.name}
                          </span>
                        {/each}
                      </div>
                    {/if}

                    {#if thinking && isExpanded}
                      <div class="bg-surface-muted mt-3 rounded border-l-2 border-blue-500 p-3">
                        <div class="text-fg-muted mb-2 flex items-center gap-1 text-xs font-semibold">
                          <span class="i-ri-brain-line"></span>
                          Thinking
                        </div>
                        <div class="text-fg-dim font-mono text-sm whitespace-pre-wrap">
                          {thinking.slice(0, 500)}{thinking.length > 500 ? "..." : ""}
                        </div>
                      </div>
                    {/if}

                    {#if isExpanded && event.raw_payload && Object.keys(event.raw_payload).length > 0}
                      <div class="bg-surface-muted mt-3 rounded p-2">
                        <div class="mb-1 flex items-center justify-between">
                          <div class="text-fg-muted text-xs font-semibold">Raw Data</div>
                          <button
                            class="text-blue hover:text-blue-bright z-10 flex items-center gap-1 text-xs"
                            onclick={(e) => {
                              e.stopPropagation();
                              onSelectEvent?.(event);
                            }}
                            type="button">
                            <span class="i-ri-eye-line"></span>
                            Inspect
                          </button>
                        </div>
                        <pre class="text-fg-dim overflow-x-auto text-xs"><code
                            >{JSON.stringify(event.raw_payload, null, 2).slice(0, 1000)}</code></pre>
                      </div>
                    {/if}
                  </div>
                  <button
                    class="text-fg-dim hover:text-blue z-10 p-1 opacity-0 transition-opacity group-hover:opacity-100"
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
  {/if}
</div>
