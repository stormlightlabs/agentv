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

<div class="flex h-full flex-col">
  <div class="border-surface-muted bg-surface-soft flex items-center justify-between border-b px-4 py-2">
    <div class="flex items-center gap-2 text-sm">
      {#if isLive}
        <span class="relative flex h-2.5 w-2.5">
          <span class="bg-green absolute inline-flex h-full w-full animate-ping rounded-full opacity-75"></span>
          <span class="bg-green relative inline-flex h-2.5 w-2.5 rounded-full"></span>
        </span>
        <span class="text-green font-medium">Live</span>
      {:else}
        <span class="bg-fg-dim h-2.5 w-2.5 rounded-full"></span>
        <span class="text-fg-dim">Idle</span>
      {/if}
      <span class="text-fg-muted text-xs">{eventBuffer.length} events</span>
    </div>
    <button
      class="rounded border px-2 py-1 text-xs transition-colors {autoScroll
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
      <div class="text-fg-dim flex h-full items-center justify-center">
        <p>Waiting for events...</p>
      </div>
    {:else}
      <div class="space-y-0.5 p-2">
        {#each eventBuffer as event (event.id)}
          <div class="hover:bg-surface-soft group flex gap-2 rounded px-2 py-1 transition-colors">
            <span class="text-fg-muted w-[72px] shrink-0">{formatTime(event.timestamp)}</span>
            <span class="shrink-0 rounded px-1.5 py-0.5 text-xs font-medium {getKindBadgeClass(event.kind)}">
              {event.kind}
            </span>
            <span class="w-[68px] shrink-0 text-xs font-semibold uppercase {getRoleClass(event.role)}">
              {getRoleLabel(event.role)}
            </span>
            <span class="text-fg min-w-0 truncate">
              {event.content?.slice(0, 200).replaceAll("\n", " ") ?? ""}
            </span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
