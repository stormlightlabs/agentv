<script lang="ts">
  import type { EventData, StreamingEventPayload } from "$lib/types";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onDestroy, onMount, tick } from "svelte";

  type Props = { sessionId: string; initialEvents: EventData[] };

  let { sessionId, initialEvents }: Props = $props();

  const MAX_BUFFER = 1000;

  let eventBuffer = $derived<EventData[]>([...initialEvents].slice(-MAX_BUFFER));
  let autoScroll = $state(true);
  let lastEventTime = $state<number>(Date.now());
  let scrollContainer: HTMLDivElement | undefined = $state();
  let unlisten: UnlistenFn | null = null;

  let isLive = $derived(Date.now() - lastEventTime < 10_000);

  function formatTime(timestamp: string): string {
    return new Date(timestamp).toLocaleTimeString("en-US", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }

  function getKindBadgeClass(kind: string): string {
    switch (kind) {
      case "message": {
        return "bg-blue/20 text-blue";
      }
      case "tool_call": {
        return "bg-purple/20 text-purple";
      }
      case "tool_result": {
        return "bg-green/20 text-green";
      }
      case "error": {
        return "bg-red/20 text-red";
      }
      case "system": {
        return "bg-fg-dim/20 text-fg-dim";
      }
      default: {
        return "bg-surface-muted text-fg-dim";
      }
    }
  }

  function getRoleLabel(role: string | null): string {
    return role ?? "system";
  }

  function getRoleClass(role: string | null): string {
    switch (role) {
      case "user": {
        return "text-green";
      }
      case "assistant": {
        return "text-blue";
      }
      default: {
        return "text-fg-dim";
      }
    }
  }

  async function scrollToBottom() {
    if (!autoScroll || !scrollContainer) return;
    await tick();
    scrollContainer.scrollTop = scrollContainer.scrollHeight;
  }

  function handleScroll() {
    if (!scrollContainer) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
    const atBottom = scrollHeight - scrollTop - clientHeight < 50;
    autoScroll = atBottom;
  }

  onMount(async () => {
    unlisten = await listen<StreamingEventPayload>("agent-events", async (event) => {
      const payload = event.payload;
      if (payload.session_external_id !== sessionId) return;

      lastEventTime = Date.now();
      const newEvents = [...eventBuffer, ...payload.events];
      eventBuffer = newEvents.slice(-MAX_BUFFER);

      await scrollToBottom();
    });

    await scrollToBottom();
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  $effect(() => {
    eventBuffer = [...initialEvents].slice(-MAX_BUFFER);
    lastEventTime = Date.now();
  });
</script>

<div class="flex flex-col h-full">
  <div class="flex items-center justify-between px-4 py-2 border-b border-surface-muted bg-surface-soft">
    <div class="flex items-center gap-2 text-sm">
      {#if isLive}
        <span class="relative flex h-2.5 w-2.5">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-green opacity-75"></span>
          <span class="relative inline-flex rounded-full h-2.5 w-2.5 bg-green"></span>
        </span>
        <span class="text-green font-medium">Live</span>
      {:else}
        <span class="h-2.5 w-2.5 rounded-full bg-fg-dim"></span>
        <span class="text-fg-dim">Idle</span>
      {/if}
      <span class="text-fg-muted text-xs">{eventBuffer.length} events</span>
    </div>
    <button
      class="px-2 py-1 text-xs border rounded transition-colors {autoScroll
        ? 'border-green text-green bg-green/10'
        : 'border-surface-muted text-fg-dim hover:border-fg-dim'}"
      onclick={() => {
        autoScroll = !autoScroll;
        if (autoScroll) scrollToBottom();
      }}>
      {autoScroll ? "Auto-scroll on" : "Auto-scroll off"}
    </button>
  </div>

  <div
    class="flex-1 overflow-y-auto font-mono text-xs leading-relaxed"
    bind:this={scrollContainer}
    onscroll={handleScroll}>
    {#if eventBuffer.length === 0}
      <div class="flex items-center justify-center h-full text-fg-dim">
        <p>Waiting for events...</p>
      </div>
    {:else}
      <div class="p-2 space-y-0.5">
        {#each eventBuffer as event (event.id)}
          <div class="flex gap-2 py-1 px-2 rounded hover:bg-surface-soft transition-colors group">
            <span class="text-fg-muted shrink-0 w-[72px]">{formatTime(event.timestamp)}</span>
            <span class="shrink-0 px-1.5 py-0.5 rounded text-2xs font-medium {getKindBadgeClass(event.kind)}">
              {event.kind}
            </span>
            <span class="shrink-0 w-[68px] font-semibold uppercase text-2xs {getRoleClass(event.role)}">
              {getRoleLabel(event.role)}
            </span>
            <span class="text-fg truncate min-w-0">
              {event.content?.slice(0, 200).replaceAll('\n', " ") ?? ""}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
