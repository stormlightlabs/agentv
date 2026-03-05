<script lang="ts">
  import { keyboardStore } from "$lib/stores/keyboard.svelte";
  import SessionViewer from "$lib/components/SessionViewer.svelte";
  import SessionList from "$lib/components/SessionList.svelte";
  import WelcomeScreen from "$lib/components/WelcomeScreen.svelte";
  import type { EventData, SessionData } from "$lib/types";
  import HomeSessionEmptyState from "./HomeSessionEmptyState.svelte";

  type SessionsTabState = {
    isNarrowLayout: boolean;
    sidebarWidth: number;
    filteredSessions: SessionData[];
    sessions: SessionData[];
    selectedSession: SessionData | null;
    events: EventData[];
    lastIngestTime: Date | null;
    loading: boolean;
  };

  type SessionsTabActions = {
    onStartResizing?: (event: MouseEvent) => void;
    onResizerKeydown?: (event: KeyboardEvent) => void;
    onSelectSession: (session: SessionData) => void;
    onGetStarted: () => void;
    onSelectEvent?: (event: EventData) => void;
    onOpenSessionMeta?: () => void;
    onFollowLatestSession?: () => void;
  };

  type Props = {
    state: SessionsTabState;
    actions: SessionsTabActions;
  };

  const minSidebarWidth = 350;

  let { state, actions }: Props = $props();
</script>

<div class="flex h-full overflow-hidden">
  {#if !state.isNarrowLayout}
    <aside
      class="bg-surface-soft border-surface-muted relative flex flex-col overflow-hidden border-r"
      style="width: {state.sidebarWidth}px; min-width: {minSidebarWidth}px;">
      <button
        class="hover:bg-blue/50 absolute top-0 right-0 z-10 h-full w-1 cursor-col-resize border-none bg-transparent p-0 transition-colors"
        onmousedown={actions.onStartResizing}
        onkeydown={actions.onResizerKeydown}
        type="button"
        aria-label="Resize sidebar">
      </button>

      <div class="flex-1 overflow-hidden">
        <SessionList sessions={state.filteredSessions} selectedSession={state.selectedSession} onSelect={actions.onSelectSession} />
      </div>

      <div class="border-surface-muted bg-surface text-fg-dim border-t p-2 text-xs">
        <div class="flex items-center justify-between">
          <span>{state.filteredSessions.length} shown</span>
          <span>{state.sessions.length} total</span>
        </div>
        {#if state.lastIngestTime}
          <div class="mt-1">Last update: {state.lastIngestTime.toLocaleTimeString()}</div>
        {/if}
      </div>
    </aside>
  {/if}

  <main class="flex flex-1 flex-col overflow-hidden">
    {#if state.sessions.length === 0 && !state.loading}
      <WelcomeScreen onGetStarted={actions.onGetStarted} />
    {:else if state.selectedSession}
      <SessionViewer
        session={state.selectedSession}
        events={state.events}
        onSelectEvent={actions.onSelectEvent}
        onOpenDrawer={actions.onOpenSessionMeta} />
    {:else}
      <HomeSessionEmptyState onOpenCommandPalette={keyboardStore.openCommandPalette} onFollowLatestSession={actions.onFollowLatestSession} />
    {/if}
  </main>
</div>
