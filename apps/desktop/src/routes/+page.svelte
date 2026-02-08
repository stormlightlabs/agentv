<script lang="ts">
  import { browser } from "$app/environment";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import AnalyticsPanel from "$lib/components/AnalyticsPanel.svelte";
  import CommandPalette from "$lib/components/CommandPalette.svelte";
  import EventInspector from "$lib/components/EventInspector.svelte";
  import IngestStatusPanel from "$lib/components/IngestStatusPanel.svelte";
  import SearchPanel from "$lib/components/SearchPanel.svelte";
  import SessionList from "$lib/components/SessionList.svelte";
  import SessionViewer from "$lib/components/SessionViewer.svelte";
  import Toast from "$lib/components/Toast.svelte";
  import WelcomeScreen from "$lib/components/WelcomeScreen.svelte";
  import {
    bookmarkStore,
    getBookmarkColor,
    getBookmarkDescription,
    getBookmarkIcon,
    type Bookmark,
  } from "$lib/stores/bookmarks.svelte";
  import { filterStore, syncFiltersFromURL, updateURLFromFilters } from "$lib/stores/filters.svelte";
  import { keyboardStore, handleKeyboardEvent, registerShortcut } from "$lib/stores/keyboard.svelte";
  import { logInfo } from "$lib/stores/logger.svelte";
  import { useToast } from "$lib/stores/toast.svelte";
  import type { EventData, IngestResult, SessionData } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy, onMount } from "svelte";
  import { fade, slide } from "svelte/transition";

  const toast = useToast();

  let bookmarksOpen = $state(false);
  let selectedEvent = $state<EventData | null>(null);
  let showEventInspector = $state(false);
  let showSessionDrawer = $state(false);

  let sidebarWidth = $state(500);
  let isResizing = $state(false);
  let minSidebarWidth = 350;
  let maxSidebarWidth = 800;

  let sessions = $state<SessionData[]>([]);
  let selectedSession = $state<SessionData | null>(null);
  let events = $state<EventData[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let activeTab = $state<"sessions" | "search" | "analytics" | "status">("sessions");
  let ingestLoading = $state(false);
  let lastIngestTime = $state<Date | null>(null);
  let newSessionsAvailable = $state(false);
  let autoRefreshEnabled = $state(true);
  let refreshInterval: number | null = null;

  const sources = [
    { id: "claude", name: "Claude", color: "blue" },
    { id: "codex", name: "Codex", color: "green" },
    { id: "opencode", name: "OpenCode", color: "purple" },
    { id: "crush", name: "Crush", color: "yellow" },
  ];

  async function loadSessions() {
    try {
      loading = true;
      error = null;
      sessions = await invoke<SessionData[]>("list_sessions");
      newSessionsAvailable = false;
    } catch (e) {
      error = String(e);
      toast.error(`Failed to load sessions: ${e}`);
    } finally {
      loading = false;
    }
  }

  async function checkForNewSessions() {
    if (!autoRefreshEnabled) return;

    try {
      const hasNewSessions = await invoke<boolean>("check_for_new_sessions");
      if (hasNewSessions && !newSessionsAvailable) {
        newSessionsAvailable = true;
        toast.info("New sessions available - click refresh to load");
      }
    } catch (e) {
      console.error("Failed to check for new sessions:", e);
    }
  }

  function startAutoRefresh() {
    if (refreshInterval) return;

    refreshInterval = window.setInterval(() => {
      checkForNewSessions();
    }, 30000);
  }

  function stopAutoRefresh() {
    if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }

  function toggleAutoRefresh() {
    autoRefreshEnabled = !autoRefreshEnabled;
    if (autoRefreshEnabled) {
      startAutoRefresh();
      toast.info("Auto-refresh enabled");
    } else {
      stopAutoRefresh();
      toast.info("Auto-refresh disabled");
    }
  }

  async function selectSession(session: SessionData) {
    selectedSession = session;
    filterStore.setFilter("sessionId", session.id);
    showSessionDrawer = false;
    try {
      events = await invoke<EventData[]>("get_session_events", { sessionId: session.id });
      logInfo("Session selected", { sessionId: session.id, eventCount: events.length });
    } catch (e) {
      console.error("Failed to load events:", e);
      toast.error(`Failed to load events: ${e}`);
      events = [];
    }
  }

  function startResizing(e: MouseEvent) {
    isResizing = true;
    e.preventDefault();
  }

  function handleResize(e: MouseEvent) {
    if (!isResizing) return;
    const newWidth = e.clientX;
    if (newWidth >= minSidebarWidth && newWidth <= maxSidebarWidth) {
      sidebarWidth = newWidth;
    }
  }

  function stopResizing() {
    isResizing = false;
  }

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast.success("Copied to clipboard");
    } catch (e) {
      toast.error("Failed to copy");
    }
  }

  async function selectSessionById(sessionId: string) {
    const session = sessions.find((s) => s.id === sessionId);
    if (session) {
      await selectSession(session);
      activeTab = "sessions";
    }
  }

  function selectEvent(event: EventData) {
    selectedEvent = event;
    showEventInspector = true;
    filterStore.setFilter("eventId", event.id);
    logInfo("Event selected", { eventId: event.id });
  }

  async function ingestSource(sourceId: string) {
    ingestLoading = true;
    error = null;

    try {
      const result = await invoke<IngestResult>("ingest_source", { source: sourceId });
      lastIngestTime = new Date();

      if (result.imported > 0) {
        toast.success(`Imported ${result.imported} sessions from ${sourceId} in ${result.duration_ms}ms`);
      } else if (result.failed > 0) {
        toast.error(`Failed to import ${result.failed} sessions from ${sourceId}`);
      } else {
        toast.info(`No new sessions found in ${sourceId}`);
      }

      await loadSessions();
    } catch (e) {
      error = String(e);
      toast.error(`Failed to ingest ${sourceId}: ${e}`);
    } finally {
      ingestLoading = false;
    }
  }

  async function ingestAllSources() {
    ingestLoading = true;
    error = null;

    try {
      const results = await invoke<IngestResult[]>("ingest_all_sources");
      lastIngestTime = new Date();

      const totalImported = results.reduce((sum, r) => sum + r.imported, 0);
      const totalFailed = results.reduce((sum, r) => sum + r.failed, 0);

      if (totalImported > 0) {
        toast.success(`Imported ${totalImported} sessions from all sources`);
      } else if (totalFailed > 0) {
        toast.error(`Failed to import ${totalFailed} sessions`);
      } else {
        toast.info("No new sessions found");
      }

      await loadSessions();
    } catch (e) {
      error = String(e);
      toast.error(`Failed to ingest all sources: ${e}`);
    } finally {
      ingestLoading = false;
    }
  }

  function bookmarkCurrentSession() {
    if (selectedSession) {
      bookmarkStore.add({
        name: selectedSession.title || selectedSession.external_id.slice(0, 8),
        type: "session",
        description: selectedSession.project || undefined,
        data: { sessionId: selectedSession.id },
      });
      toast.success("Session bookmarked");
    }
  }

  function bookmarkCurrentFilters() {
    const filters = {
      source: filterStore.state.source,
      project: filterStore.state.project,
      kind: filterStore.state.kind,
      role: filterStore.state.role,
      tool: filterStore.state.tool,
      since: filterStore.state.since,
      until: filterStore.state.until,
    };
    bookmarkStore.add({
      name: "Filter Set",
      type: "filter",
      description: Object.entries(filters)
        .filter(([, v]) => v)
        .map(([k, v]) => `${k}: ${v}`)
        .join(", ")
        .slice(0, 50),
      data: { filters },
    });
    toast.success("Filters bookmarked");
  }

  function applyBookmark(bookmark: Bookmark) {
    switch (bookmark.type) {
      case "session":
        if (bookmark.data.sessionId) {
          selectSessionById(bookmark.data.sessionId);
        }
        break;
      case "filter":
        if (bookmark.data.filters) {
          Object.entries(bookmark.data.filters).forEach(([key, value]) => {
            if (value) filterStore.setFilter(key as keyof typeof filterStore.state, value);
          });
        }
        break;
      case "search":
        if (bookmark.data.query) {
          filterStore.setFilter("query", bookmark.data.query);
          activeTab = "search";
        }
        break;
    }
    bookmarksOpen = false;
  }

  function deleteBookmark(id: string) {
    bookmarkStore.remove(id);
  }

  function setupKeyboardShortcuts() {
    if (!browser) return;

    registerShortcut({
      key: "k",
      modifiers: { meta: true },
      description: "Open command palette",
      scope: "global",
      handler: keyboardStore.openCommandPalette,
    });

    registerShortcut({
      key: "1",
      modifiers: { meta: true },
      description: "Go to sessions",
      scope: "global",
      handler: () => (activeTab = "sessions"),
    });

    registerShortcut({
      key: "2",
      modifiers: { meta: true },
      description: "Go to search",
      scope: "global",
      handler: () => (activeTab = "search"),
    });

    registerShortcut({
      key: "3",
      modifiers: { meta: true },
      description: "Go to analytics",
      scope: "global",
      handler: () => (activeTab = "analytics"),
    });

    registerShortcut({
      key: "4",
      modifiers: { meta: true },
      description: "Go to status",
      scope: "global",
      handler: () => (activeTab = "status"),
    });

    registerShortcut({
      key: "r",
      modifiers: { meta: true },
      description: "Refresh sessions",
      scope: "global",
      handler: loadSessions,
    });

    registerShortcut({
      key: "b",
      modifiers: { meta: true },
      description: "Toggle bookmarks",
      scope: "global",
      handler: () => (bookmarksOpen = !bookmarksOpen),
    });

    if (selectedSession) {
      registerShortcut({
        key: "d",
        modifiers: { meta: true },
        description: "Bookmark current session",
        scope: "global",
        handler: bookmarkCurrentSession,
      });
    }
  }

  function updateCommandPalette() {
    const commands: typeof keyboardStore.commandPaletteItems = [
      {
        id: "sessions",
        title: "Go to Sessions",
        subtitle: "View all sessions",
        icon: "i-ri-chat-3-line",
        category: "navigation",
        shortcut: "Cmd+1",
        action: () => (activeTab = "sessions"),
      },
      {
        id: "search",
        title: "Go to Search",
        subtitle: "Search across events",
        icon: "i-ri-search-line",
        category: "navigation",
        shortcut: "Cmd+2",
        action: () => (activeTab = "search"),
      },
      {
        id: "analytics",
        title: "Go to Analytics",
        subtitle: "View charts and statistics",
        icon: "i-ri-bar-chart-line",
        category: "navigation",
        shortcut: "Cmd+3",
        action: () => (activeTab = "analytics"),
      },
      {
        id: "status",
        title: "Go to Status",
        subtitle: "View ingest status",
        icon: "i-ri-heart-pulse-line",
        category: "navigation",
        shortcut: "Cmd+4",
        action: () => (activeTab = "status"),
      },
      {
        id: "refresh",
        title: "Refresh Sessions",
        subtitle: "Reload all sessions",
        icon: "i-ri-refresh-line",
        category: "action",
        shortcut: "Cmd+R",
        action: loadSessions,
      },
      {
        id: "ingest-all",
        title: "Ingest All Sources",
        subtitle: "Import from all sources",
        icon: "i-ri-download-cloud-line",
        category: "action",
        action: ingestAllSources,
      },
      {
        id: "toggle-bookmarks",
        title: "Toggle Bookmarks",
        subtitle: "Show/hide bookmarks panel",
        icon: "i-ri-bookmark-line",
        category: "action",
        shortcut: "Cmd+B",
        action: () => (bookmarksOpen = !bookmarksOpen),
      },
      ...bookmarkStore.bookmarks.map((bookmark: Bookmark) => ({
        id: `bookmark-${bookmark.id}`,
        title: bookmark.name,
        subtitle: getBookmarkDescription(bookmark),
        icon: getBookmarkIcon(bookmark.type),
        category: "view" as const,
        action: () => applyBookmark(bookmark),
      })),
    ];

    keyboardStore.setCommandPaletteItems(commands);
  }

  function handleUrlParams() {
    if (!browser) return;

    syncFiltersFromURL(page.url.searchParams);

    const sessionId = page.url.searchParams.get("session");
    if (sessionId) {
      selectSessionById(sessionId);
    }

    const tab = page.url.searchParams.get("tab");
    if (tab && ["sessions", "search", "analytics", "status"].includes(tab)) {
      activeTab = tab as typeof activeTab;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
      return;
    }

    handleKeyboardEvent(event);
  }

  onMount(() => {
    bookmarkStore.init();
    loadSessions();
    startAutoRefresh();
    setupKeyboardShortcuts();
    handleUrlParams();

    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("mousemove", handleResize);
    window.addEventListener("mouseup", stopResizing);

    return () => {
      stopAutoRefresh();
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("mousemove", handleResize);
      window.removeEventListener("mouseup", stopResizing);
    };
  });

  onDestroy(() => {
    stopAutoRefresh();
  });

  $effect(() => {
    updateCommandPalette();
  });

  $effect(() => {
    if (browser) {
      const params = new URLSearchParams();
      if (activeTab !== "sessions") params.set("tab", activeTab);
      if (filterStore.state.query) params.set("q", filterStore.state.query);
      if (filterStore.state.sessionId) params.set("session", filterStore.state.sessionId);
      if (filterStore.state.source) params.set("source", filterStore.state.source);
      if (filterStore.state.project) params.set("project", filterStore.state.project);
      if (filterStore.state.kind) params.set("kind", filterStore.state.kind);
      if (filterStore.state.role) params.set("role", filterStore.state.role);
      if (filterStore.state.since) params.set("since", filterStore.state.since);

      const url = params.toString() ? `?${params.toString()}` : "/";
      goto(url, { replaceState: true, keepFocus: true });
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="fixed top-4 right-4 z-50 flex flex-col gap-2">
  {#each toast.notifications as notification (notification.id)}
    <Toast {notification} onDismiss={toast.removeToast} />
  {/each}
</div>

<CommandPalette />

{#if bookmarksOpen}
  <div
    class="fixed inset-0 z-40 bg-black/50"
    onclick={() => (bookmarksOpen = false)}
    onkeydown={(e) => {
      if (e.key === "Escape") bookmarksOpen = false;
    }}
    role="dialog"
    aria-label="Bookmarks"
    tabindex="-1"
    transition:fade={{ duration: 200 }}>
    <div
      class="absolute right-0 top-0 h-full w-80 bg-bg border-l border-bg-muted shadow-xl"
      onclick={(e) => e.stopPropagation()}
      transition:slide={{ axis: "x", duration: 200 }}>
      <div class="flex flex-col h-full">
        <div class="flex items-center justify-between p-4 border-b border-bg-muted">
          <h2 class="text-lg font-semibold text-fg m-0">Bookmarks</h2>
          <button class="p-2 text-fg-dim hover:text-fg transition-colors" onclick={() => (bookmarksOpen = false)}>
            <span class="i-ri-close-line"></span>
          </button>
        </div>

        <div class="flex-1 overflow-y-auto p-4 space-y-2">
          {#if bookmarkStore.bookmarks.length === 0}
            <div class="text-center text-fg-dim py-8">
              <div class="i-ri-bookmark-line text-3xl mb-2 opacity-50"></div>
              <p>No bookmarks yet</p>
              <p class="text-sm">Use Cmd+D to bookmark sessions</p>
            </div>
          {:else}
            {#each bookmarkStore.bookmarks as bookmark (bookmark.id)}
              <div
                class="group flex items-start gap-3 p-3 bg-bg-soft rounded border border-bg-muted hover:border-blue transition-colors"
                onclick={() => applyBookmark(bookmark)}
                role="button"
                tabindex="0"
                onkeydown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    applyBookmark(bookmark);
                  }
                }}>
                <span class="{getBookmarkIcon(bookmark.type)} {getBookmarkColor(bookmark.type)} mt-0.5"></span>
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium text-fg truncate">{bookmark.name}</div>
                  {#if bookmark.description}
                    <div class="text-xs text-fg-dim truncate">{bookmark.description}</div>
                  {/if}
                </div>
                <button
                  class="opacity-0 group-hover:opacity-100 p-1 text-fg-dim hover:text-red transition-all"
                  onclick={(e) => {
                    e.stopPropagation();
                    deleteBookmark(bookmark.id);
                  }}
                  aria-label="Delete bookmark"
                  type="button">
                  <span class="i-ri-delete-bin-line"></span>
                </button>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- TODO: Modal component -->
{#if showEventInspector && selectedEvent}
  <div
    class="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4"
    onclick={() => (showEventInspector = false)}
    onkeydown={(e) => {
      if (e.key === "Escape") showEventInspector = false;
    }}
    role="dialog"
    aria-label="Event inspector"
    tabindex="-1"
    transition:fade={{ duration: 150 }}>
    <div
      class="w-full max-w-4xl h-[80vh] bg-bg rounded-lg shadow-2xl overflow-hidden"
      onclick={(e) => e.stopPropagation()}
      transition:slide={{ duration: 150 }}>
      <EventInspector
        event={selectedEvent}
        onCopyId={() => toast.success("ID copied")}
        onCopyPayload={() => toast.success("Payload copied")}
        onNavigateParent={(parentId) => {
          if (parentId) {
            toast.info(`Navigate to parent: ${parentId}`);
          }
        }} />
    </div>
  </div>
{/if}

{#if showSessionDrawer && selectedSession}
  <div
    class="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4"
    onclick={() => (showSessionDrawer = false)}
    onkeydown={(e) => {
      if (e.key === "Escape") showSessionDrawer = false;
    }}
    role="dialog"
    aria-label="Session details"
    tabindex="-1"
    transition:fade={{ duration: 150 }}>
    <div
      class="w-full max-w-5xl h-[85vh] bg-bg rounded-lg shadow-2xl overflow-hidden flex flex-col"
      onclick={(e) => e.stopPropagation()}
      transition:slide={{ duration: 150 }}>
      <div class="flex items-center justify-between px-6 py-4 border-b border-bg-muted bg-bg-soft">
        <div class="flex items-center gap-3">
          <h2 class="text-xl font-semibold text-fg m-0">
            {selectedSession.title || "Untitled Session"}
          </h2>
          <span class="px-2 py-0.5 bg-bg-muted rounded text-2xs text-fg-dim uppercase">
            {selectedSession.source}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <button
            class="px-3 py-1.5 bg-bg border border-bg-muted rounded text-sm text-fg hover:border-blue hover:text-blue transition-colors flex items-center gap-1"
            onclick={() => copyToClipboard(JSON.stringify(selectedSession, null, 2))}
            type="button">
            <span class="i-ri-file-copy-line"></span>
            Copy Session JSON
          </button>
          <button
            class="p-2 text-fg-dim hover:text-fg transition-colors"
            onclick={() => (showSessionDrawer = false)}
            type="button">
            <span class="i-ri-close-line text-xl"></span>
          </button>
        </div>
      </div>
      <div class="flex-1 overflow-auto p-6">
        <div class="mb-6 grid grid-cols-3 gap-4 text-sm">
          <div class="p-3 bg-bg-soft rounded border border-bg-muted">
            <div class="text-xs text-fg-muted mb-1">Session ID</div>
            <div class="text-fg font-mono text-xs">{selectedSession.id}</div>
          </div>
          <div class="p-3 bg-bg-soft rounded border border-bg-muted">
            <div class="text-xs text-fg-muted mb-1">External ID</div>
            <div class="text-fg font-mono text-xs">{selectedSession.external_id}</div>
          </div>
          <div class="p-3 bg-bg-soft rounded border border-bg-muted">
            <div class="text-xs text-fg-muted mb-1">Project</div>
            <div class="text-fg">{selectedSession.project || "No project"}</div>
          </div>
          <div class="p-3 bg-bg-soft rounded border border-bg-muted">
            <div class="text-xs text-fg-muted mb-1">Created</div>
            <div class="text-fg">{new Date(selectedSession.created_at).toLocaleString()}</div>
          </div>
          <div class="p-3 bg-bg-soft rounded border border-bg-muted">
            <div class="text-xs text-fg-muted mb-1">Updated</div>
            <div class="text-fg">{new Date(selectedSession.updated_at).toLocaleString()}</div>
          </div>
          <div class="p-3 bg-bg-soft rounded border border-bg-muted">
            <div class="text-xs text-fg-muted mb-1">Events</div>
            <div class="text-fg">{events.length} events</div>
          </div>
        </div>
        <div class="bg-bg-soft rounded border border-bg-muted overflow-hidden">
          <div class="px-4 py-2 border-b border-bg-muted bg-bg-muted/50 flex items-center justify-between">
            <span class="text-sm font-semibold text-fg">Full Session Data</span>
            <span class="text-2xs text-fg-dim">JSON</span>
          </div>
          <pre class="p-4 text-sm text-fg-dim overflow-x-auto max-h-[50vh]"><code
              >{JSON.stringify(selectedSession, null, 2)}</code></pre>
        </div>
      </div>
    </div>
  </div>
{/if}

<div class="flex h-screen overflow-hidden">
  <aside
    class="bg-bg-soft border-r border-bg-muted flex flex-col overflow-hidden relative"
    style="width: {sidebarWidth}px; min-width: {minSidebarWidth}px;">
    <div
      class="absolute top-0 right-0 w-1 h-full cursor-col-resize hover:bg-blue/50 transition-colors z-10"
      onmousedown={startResizing}
      role="separator"
      aria-label="Resize sidebar">
    </div>
    <div class="p-4 border-b border-bg-muted flex flex-col gap-3">
      <div class="flex items-center justify-between">
        <h1 class="m-0 text-xl font-semibold text-fg">Agent V</h1>
        <button
          class="p-2 text-fg-dim hover:text-fg transition-colors relative"
          onclick={() => (bookmarksOpen = !bookmarksOpen)}
          title="Bookmarks (Cmd+B)">
          <span class="i-ri-bookmark-line"></span>
          {#if bookmarkStore.bookmarks.length > 0}
            <span class="absolute top-1 right-1 w-2 h-2 bg-blue rounded-full" aria-hidden="true"> </span>
          {/if}
        </button>
      </div>

      {#if newSessionsAvailable}
        <div
          class="px-3 py-2 bg-yellow/20 border border-yellow rounded text-xs text-yellow flex items-center justify-between"
          transition:slide>
          <div class="flex items-center gap-2">
            <span class="i-ri-notification-3-line animate-pulse"></span>
            <span>New sessions available</span>
          </div>
          <button
            class="bg-transparent border-none p-0 text-yellow font-semibold cursor-pointer hover:underline"
            onclick={ingestAllSources}>
            Refresh
          </button>
        </div>
      {/if}

      <button
        class="px-4 py-2 bg-blue text-bg border-none rounded font-inherit text-sm cursor-pointer transition-colors hover:not-disabled:bg-blue-bright disabled:opacity-50 disabled:cursor-not-allowed"
        onclick={ingestAllSources}
        disabled={ingestLoading}>
        {#if ingestLoading}
          <span class="i-ri-loader-4-line animate-spin mr-1"></span>
          Ingesting...
        {:else}
          <span class="i-ri-refresh-line mr-1"></span>
          Refresh All
        {/if}
      </button>

      <div class="flex flex-wrap gap-1">
        {#each sources as source}
          <button
            class="px-2 py-1 bg-bg border border-bg-muted rounded text-2xs text-fg cursor-pointer transition-all hover:border-{source.color} hover:text-{source.color} disabled:opacity-50"
            onclick={() => ingestSource(source.id)}
            disabled={ingestLoading}
            title="Refresh {source.name}">
            {source.name}
          </button>
        {/each}
      </div>

      <div class="flex items-center justify-between text-2xs text-fg-dim">
        <button
          class="flex items-center gap-1.5 cursor-pointer bg-transparent border-none p-0 text-inherit hover:text-fg"
          onclick={toggleAutoRefresh}>
          {#if autoRefreshEnabled}
            <span class="i-ri-checkbox-circle-line text-green"></span>
            <span>Auto-refresh on</span>
          {:else}
            <span class="i-ri-checkbox-blank-circle-line"></span>
            <span>Auto-refresh off</span>
          {/if}
        </button>
        <button
          class="cursor-pointer bg-transparent border-none p-0 text-inherit hover:text-fg flex items-center gap-1"
          onclick={keyboardStore.openCommandPalette}>
          <span class="i-ri-command-line"></span>
          <span>Cmd+K</span>
        </button>
      </div>
    </div>

    <div class="flex border-b border-bg-muted bg-bg">
      <button
        class="flex-1 px-3 py-3 bg-transparent border-none border-b-2 border-transparent text-fg-dim font-inherit text-sm cursor-pointer transition-all hover:text-fg hover:bg-bg-soft"
        class:active={activeTab === "sessions"}
        class:text-blue={activeTab === "sessions"}
        class:border-b-blue={activeTab === "sessions"}
        class:bg-bg-soft={activeTab === "sessions"}
        onclick={() => (activeTab = "sessions")}>
        Sessions
      </button>
      <button
        class="flex-1 px-3 py-3 bg-transparent border-none border-b-2 border-transparent text-fg-dim font-inherit text-sm cursor-pointer transition-all hover:text-fg hover:bg-bg-soft"
        class:active={activeTab === "search"}
        class:text-blue={activeTab === "search"}
        class:border-b-blue={activeTab === "search"}
        class:bg-bg-soft={activeTab === "search"}
        onclick={() => (activeTab = "search")}>
        Search
      </button>
      <button
        class="flex-1 px-3 py-3 bg-transparent border-none border-b-2 border-transparent text-fg-dim font-inherit text-sm cursor-pointer transition-all hover:text-fg hover:bg-bg-soft"
        class:active={activeTab === "analytics"}
        class:text-blue={activeTab === "analytics"}
        class:border-b-blue={activeTab === "analytics"}
        class:bg-bg-soft={activeTab === "analytics"}
        onclick={() => (activeTab = "analytics")}>
        Analytics
      </button>
      <button
        class="flex-1 px-3 py-3 bg-transparent border-none border-b-2 border-transparent text-fg-dim font-inherit text-sm cursor-pointer transition-all hover:text-fg hover:bg-bg-soft"
        class:active={activeTab === "status"}
        class:text-blue={activeTab === "status"}
        class:border-b-blue={activeTab === "status"}
        class:bg-bg-soft={activeTab === "status"}
        onclick={() => (activeTab = "status")}>
        Status
      </button>
    </div>

    {#if error}
      <div class="mx-4 my-2 p-2 bg-red text-bg rounded text-xs" transition:fade>
        {error}
      </div>
    {/if}

    <div class="flex-1 overflow-hidden">
      {#if activeTab === "sessions"}
        <SessionList {sessions} {selectedSession} onSelect={selectSession} />
      {:else if activeTab === "search"}
        <SearchPanel onSelectSession={selectSessionById} onSelectEvent={selectEvent} />
      {:else if activeTab === "analytics"}
        <AnalyticsPanel />
      {:else}
        <IngestStatusPanel onRefresh={loadSessions} />
      {/if}
    </div>

    <div class="p-2 border-t border-bg-muted bg-bg text-xs text-fg-dim">
      <div class="flex justify-between items-center">
        <span>{sessions.length} sessions</span>
        {#if lastIngestTime}
          <span>Last update: {lastIngestTime.toLocaleTimeString()}</span>
        {/if}
      </div>
    </div>
  </aside>

  <main class="flex-1 overflow-hidden flex flex-col">
    {#if sessions.length === 0 && !loading}
      <WelcomeScreen onGetStarted={ingestAllSources} />
    {:else if selectedSession}
      <SessionViewer
        session={selectedSession}
        {events}
        onSelectEvent={selectEvent}
        onOpenDrawer={() => (showSessionDrawer = true)} />
    {:else}
      <div class="flex-1 flex items-center justify-center text-fg-dim" in:fade>
        <div class="text-center">
          <div class="i-ri-chat-3-line text-4xl mb-3 opacity-50"></div>
          <p>Select a session to view details</p>
          <p class="text-sm text-fg-muted mt-2">Use Cmd+K for quick actions</p>
        </div>
      </div>
    {/if}
  </main>
</div>
