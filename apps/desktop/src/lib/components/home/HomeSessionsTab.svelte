<script lang="ts">
  import SessionList from "$lib/components/SessionList.svelte";
  import SessionViewer from "$lib/components/SessionViewer.svelte";
  import WelcomeScreen from "$lib/components/WelcomeScreen.svelte";
  import { keyboardStore } from "$lib/stores/keyboard.svelte";
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
    refreshingSessions: boolean;
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

  type Props = { state: SessionsTabState; actions: SessionsTabActions };

  const minSidebarWidth = 350;

  let { state, actions }: Props = $props();
</script>

<div class="flex h-full overflow-hidden">
  {#if !state.isNarrowLayout}
    <aside
      class="bg-surface-soft border-surface-muted relative flex shrink-0 flex-col overflow-hidden border-r"
      style="width: {state.sidebarWidth}px; min-width: {minSidebarWidth}px;">
      <button
        class="hover:bg-blue/50 absolute top-0 right-0 z-10 h-full w-1 cursor-col-resize border-none bg-transparent p-0 transition-colors"
        onmousedown={actions.onStartResizing}
        onkeydown={actions.onResizerKeydown}
        type="button"
        aria-label="Resize sidebar">
      </button>

      <div class="flex-1 overflow-hidden">
        <SessionList
          sessions={state.filteredSessions}
          selectedSession={state.selectedSession}
          onSelect={actions.onSelectSession} />
      </div>

      <div class="border-surface-muted bg-surface text-fg-dim flex items-center justify-between border-t p-2 text-xs">
        <div class="flex items-center gap-2">
          {#if state.refreshingSessions}
            <span class="text-blue flex items-center gap-1">
              <span class="flex items-center">
                <span class="i-ri-loader-4-line animate-spin"></span>
              </span>
              <span>Refreshing</span>
            </span>
          {/if}
          {#if state.lastIngestTime}
            <span>Last updated: {state.lastIngestTime.toLocaleTimeString()}</span>
          {/if}
        </div>
        <div class="flex items-center gap-2">
          <span>Loaded</span>
          <span>{state.filteredSessions.length} / {state.sessions.length} sessions</span>
        </div>
      </div>
    </aside>
  {/if}

  <main class="flex min-w-0 flex-1 flex-col overflow-hidden">
    {#if state.loading && state.sessions.length === 0}
      <div class="text-fg-dim flex flex-1 items-center justify-center px-6">
        <div class="flex flex-col items-center gap-3 text-center">
          <span class="flex items-center text-3xl">
            <span class="i-ri-loader-4-line animate-spin"></span>
          </span>
          <div class="text-fg text-base font-medium">Loading sessions</div>
          <div class="text-fg-muted text-sm">Reading your local agent history and rebuilding the current view.</div>
        </div>
      </div>
    {:else if state.sessions.length === 0}
      <WelcomeScreen onGetStarted={actions.onGetStarted} />
    {:else if state.selectedSession}
      <SessionViewer
        session={state.selectedSession}
        events={state.events}
        onSelectEvent={actions.onSelectEvent}
        onOpenDrawer={actions.onOpenSessionMeta} />
    {:else}
      <HomeSessionEmptyState
        onOpenCommandPalette={keyboardStore.openCommandPalette}
        onFollowLatestSession={actions.onFollowLatestSession} />
    {/if}
  </main>
</div>
