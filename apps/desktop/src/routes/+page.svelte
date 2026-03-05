<script lang="ts">
  import { browser } from "$app/environment";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import AnalyticsPanel from "$lib/components/AnalyticsPanel.svelte";
  import CommandPalette from "$lib/components/CommandPalette.svelte";
  import Dialog from "$lib/components/Dialog.svelte";
  import Drawer from "$lib/components/Drawer.svelte";
  import EventInspector from "$lib/components/EventInspector.svelte";
  import IngestStatusPanel from "$lib/components/IngestStatusPanel.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import SearchPanel from "$lib/components/SearchPanel.svelte";
  import SessionList from "$lib/components/SessionList.svelte";
  import SessionViewer from "$lib/components/SessionViewer.svelte";
  import Sheet from "$lib/components/Sheet.svelte";
  import SupportPanel from "$lib/components/SupportPanel.svelte";
  import Toast from "$lib/components/Toast.svelte";
  import WelcomeScreen from "$lib/components/WelcomeScreen.svelte";
  import {
    bookmarkStore,
    getBookmarkColor,
    getBookmarkDescription,
    getBookmarkIcon,
    type Bookmark,
  } from "$lib/stores/bookmarks.svelte";
  import { filterStore, syncFiltersFromURL } from "$lib/stores/filters.svelte";
  import {
    handleKeyboardEvent,
    keyboardStore,
    registerShortcut,
    type CommandPaletteItem,
  } from "$lib/stores/keyboard.svelte";
  import { logInfo } from "$lib/stores/logger.svelte";
  import { supportNudgeStore } from "$lib/stores/supportNudge.svelte";
  import { useNotifications } from "$lib/stores/notifications.svelte";
  import { useToast } from "$lib/stores/toast.svelte";
  import type {
    EventData,
    ExportFormat,
    IngestProgress,
    IngestResult,
    SessionData,
    SessionListMetricsData,
    StreamingEventPayload,
  } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onDestroy, onMount } from "svelte";
  import { fade, slide } from "svelte/transition";

  type Tab = "sessions" | "search" | "analytics" | "status" | "support";

  const toast = useToast();
  const notifications = useNotifications();

  const tabs: Array<{ id: Tab; label: string; icon: string }> = [
    { id: "sessions", label: "Sessions", icon: "i-ri-chat-3-line" },
    { id: "search", label: "Search", icon: "i-ri-search-line" },
    { id: "analytics", label: "Analytics", icon: "i-ri-bar-chart-line" },
    { id: "status", label: "Status", icon: "i-ri-heart-pulse-line" },
    { id: "support", label: "Support", icon: "i-ri-heart-line" },
  ];

  const dateRanges = [
    { label: "All time", value: "" },
    { label: "Last 24h", value: "1d" },
    { label: "Last 7d", value: "7d" },
    { label: "Last 30d", value: "30d" },
    { label: "Last 90d", value: "90d" },
  ];

  const sources = [
    { id: "claude", name: "Claude" },
    { id: "codex", name: "Codex" },
    { id: "opencode", name: "OpenCode" },
    { id: "crush", name: "Crush" },
  ];

  const minSidebarWidth = 350;
  const maxSidebarWidth = 800;
  const narrowLayoutBreakpoint = 1180;

  let unlistenAgentEvents: UnlistenFn | null = null;
  let unlistenIngestProgress: UnlistenFn | null = null;

  let bookmarksOpen = $state(false);
  let selectedEvent = $state<EventData | null>(null);
  let showEventInspector = $state(false);
  let showSessionMetaModal = $state(false);
  let showSessionListDrawer = $state(false);
  let showTopFilters = $state(false);

  let sidebarWidth = $state(500);
  let isResizing = $state(false);
  let isNarrowLayout = $state(false);

  let sessions = $state<SessionData[]>([]);
  let selectedSession = $state<SessionData | null>(null);
  let events = $state<EventData[]>([]);
  let sessionMetricsById = $state<Record<string, SessionListMetricsData>>({});

  let loading = $state(true);
  let error = $state<string | null>(null);
  let activeTab = $state<Tab>("sessions");
  let ingestLoading = $state(false);
  let lastIngestTime = $state<Date | null>(null);
  let newSessionsAvailable = $state(false);
  let autoRefreshEnabled = $state(true);
  let hasDiffOnly = $state(false);
  let errorsOnly = $state(false);

  let refreshInterval: number | null = null;
  let showSupportNudge = $state(false);
  let ingestProgress = $state<IngestProgress | null>(null);
  let progressHideTimeout: number | null = null;

  function parseRangeToMs(range: string | null): number | null {
    if (!range) return null;
    const value = Number.parseInt(range.slice(0, -1), 10);
    if (Number.isNaN(value) || value <= 0) return null;

    const suffix = range.slice(-1);
    if (suffix === "h") return value * 60 * 60 * 1000;
    if (suffix === "d") return value * 24 * 60 * 60 * 1000;
    if (suffix === "w") return value * 7 * 24 * 60 * 60 * 1000;
    if (suffix === "m") return value * 30 * 24 * 60 * 60 * 1000;
    return null;
  }

  let filteredSessions = $derived.by(() => {
    let rows = [...sessions];

    if (filterStore.state.source) {
      rows = rows.filter((session) => session.source === filterStore.state.source);
    }

    if (filterStore.state.project) {
      const projectQuery = filterStore.state.project.toLowerCase();
      rows = rows.filter((session) => (session.project ?? "").toLowerCase().includes(projectQuery));
    }

    if (filterStore.state.query.trim()) {
      const query = filterStore.state.query.trim().toLowerCase();
      rows = rows.filter((session) => {
        return (
          (session.title ?? "").toLowerCase().includes(query) ||
          session.external_id.toLowerCase().includes(query) ||
          (session.project ?? "").toLowerCase().includes(query)
        );
      });
    }

    const rangeMs = parseRangeToMs(filterStore.state.since);
    if (rangeMs) {
      const cutoff = Date.now() - rangeMs;
      rows = rows.filter((session) => new Date(session.updated_at).getTime() >= cutoff);
    }

    if (hasDiffOnly) {
      rows = rows.filter((session) => {
        const metrics = sessionMetricsById[session.id];
        return Boolean(metrics && (metrics.lines_added > 0 || metrics.lines_removed > 0));
      });
    }

    if (errorsOnly) {
      rows = rows.filter((session) => {
        const metrics = sessionMetricsById[session.id];
        return Boolean(metrics && metrics.error_count > 0);
      });
    }

    return rows;
  });

  let latestVisibleSession = $derived.by(() => {
    if (filteredSessions.length === 0) return null;
    return [...filteredSessions].sort((a, b) => b.updated_at.localeCompare(a.updated_at))[0] ?? null;
  });

  async function loadSessionMetrics() {
    try {
      const metrics = await invoke<SessionListMetricsData[]>("list_session_metrics", {
        limit: Math.max(2000, sessions.length + 100),
        offset: 0,
      });
      sessionMetricsById = Object.fromEntries(metrics.map((metric) => [metric.session_id, metric]));
    } catch (e) {
      console.error("Failed to load session metrics:", e);
      sessionMetricsById = {};
    }
  }

  async function loadSessions() {
    try {
      loading = true;
      error = null;
      sessions = await invoke<SessionData[]>("list_sessions");
      newSessionsAvailable = false;
      await loadSessionMetrics();
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
    }, 120000);
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

  function setSourceScope(source: string | null) {
    filterStore.setFilter("source", source);
  }

  function clearTopFilters() {
    filterStore.setFilter("query", "");
    filterStore.setFilter("since", null);
    hasDiffOnly = false;
    errorsOnly = false;
  }

  function handleWindowResize() {
    isNarrowLayout = window.innerWidth < narrowLayoutBreakpoint;
    if (!isNarrowLayout) {
      showSessionListDrawer = false;
    }
  }

  async function exportSelectedSession(format: ExportFormat) {
    if (!selectedSession) {
      toast.info("Select a session before exporting");
      return;
    }

    try {
      const content = await invoke<string>("export_session", { sessionId: selectedSession.id, format });
      const mime = format === "md" ? "text/markdown" : "application/json";
      const blob = new Blob([content], { type: mime });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `session-${selectedSession.external_id.slice(0, 8)}.${format}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast.success(`Exported ${format.toUpperCase()}`);
    } catch (e) {
      toast.error(`Failed to export session: ${e}`);
    }
  }

  async function selectSession(session: SessionData) {
    selectedSession = session;
    filterStore.setFilter("sessionId", session.id);
    showSessionListDrawer = false;
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
    if (isNarrowLayout) return;
    isResizing = true;
    e.preventDefault();
  }

  function handleResize(e: MouseEvent) {
    if (!isResizing || isNarrowLayout) return;
    const newWidth = e.clientX;
    if (newWidth >= minSidebarWidth && newWidth <= maxSidebarWidth) {
      sidebarWidth = newWidth;
    }
  }

  function stopResizing() {
    isResizing = false;
  }

  function handleResizerKeydown(event: KeyboardEvent) {
    if (isNarrowLayout) return;

    const step = 20;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      sidebarWidth = Math.max(minSidebarWidth, sidebarWidth - step);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      sidebarWidth = Math.min(maxSidebarWidth, sidebarWidth + step);
    } else if (event.key === "Home") {
      event.preventDefault();
      sidebarWidth = minSidebarWidth;
    } else if (event.key === "End") {
      event.preventDefault();
      sidebarWidth = maxSidebarWidth;
    }
  }

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast.success("Copied to clipboard");
    } catch {
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

  async function followLatestSession() {
    if (!latestVisibleSession) {
      toast.info("No sessions available to follow");
      return;
    }
    await selectSession(latestVisibleSession);
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
        if (!supportNudgeStore.state.firstIngestCompleted) {
          supportNudgeStore.markFirstIngestCompleted();
          supportNudgeStore.markOnboardingComplete();
          setTimeout(() => {
            if (supportNudgeStore.shouldShowNudge()) {
              showSupportNudge = true;
            }
          }, 1500);
        }
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

        if (!supportNudgeStore.state.firstIngestCompleted) {
          supportNudgeStore.markFirstIngestCompleted();
          supportNudgeStore.markOnboardingComplete();
          setTimeout(() => {
            if (supportNudgeStore.shouldShowNudge()) {
              showSupportNudge = true;
            }
          }, 1500);
        }
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
      key: "5",
      modifiers: { meta: true },
      description: "Go to support",
      scope: "global",
      handler: () => (activeTab = "support"),
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

    registerShortcut({
      key: "f",
      modifiers: { meta: true },
      description: "Toggle top filters",
      scope: "global",
      handler: () => (showTopFilters = !showTopFilters),
    });
  }

  function updateCommandPalette() {
    const commands: CommandPaletteItem[] = [
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
        id: "support",
        title: "Go to Support",
        subtitle: "Support Agent V development",
        icon: "i-ri-heart-line",
        category: "navigation",
        shortcut: "Cmd+5",
        action: () => (activeTab = "support"),
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
      {
        id: "toggle-filters",
        title: "Toggle Top Filters",
        subtitle: "Show/hide global filter panel",
        icon: "i-ri-filter-3-line",
        category: "action",
        shortcut: "Cmd+F",
        action: () => (showTopFilters = !showTopFilters),
      },
      {
        id: "bookmark-filters",
        title: "Bookmark Current Filters",
        subtitle: "Save current source/date/chips",
        icon: "i-ri-bookmark-3-line",
        category: "action",
        action: bookmarkCurrentFilters,
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
    hasDiffOnly = page.url.searchParams.get("hasDiff") === "1";
    errorsOnly = page.url.searchParams.get("errors") === "1";

    const sessionId = page.url.searchParams.get("session");
    if (sessionId) {
      selectSessionById(sessionId);
    }

    const tab = page.url.searchParams.get("tab");
    if (tab && ["sessions", "search", "analytics", "status", "support"].includes(tab)) {
      activeTab = tab as Tab;
    }

    if (filterStore.activeCount > 0 || hasDiffOnly || errorsOnly) {
      showTopFilters = true;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
      return;
    }

    handleKeyboardEvent(event);
  }

  async function setupIngestProgressListener() {
    unlistenIngestProgress = await listen<IngestProgress>("ingest-progress", (event) => {
      const p = event.payload;
      if (p.phase === "Complete") {
        if (progressHideTimeout) clearTimeout(progressHideTimeout);
        progressHideTimeout = window.setTimeout(() => {
          ingestProgress = null;
        }, 1500);
      } else {
        if (progressHideTimeout) {
          clearTimeout(progressHideTimeout);
          progressHideTimeout = null;
        }
        ingestProgress = p;
      }
    });
  }

  async function setupAgentEventListener() {
    unlistenAgentEvents = await listen<StreamingEventPayload>("agent-events", (event) => {
      const payload = event.payload;

      if (payload.is_new_session) {
        loadSessions();
        notifications.notify(`New ${payload.source} session`, `New session from ${payload.source}`);
      } else if (selectedSession && selectedSession.external_id === payload.session_external_id) {
        events = [...events, ...payload.events];
      }

      if (!payload.is_new_session && payload.events.length > 0) {
        const lastEvent = payload.events[payload.events.length - 1];
        const summary = (lastEvent.content ?? lastEvent.kind).slice(0, 80);
        notifications.notify(payload.source, summary);
      }
    });
  }

  onMount(() => {
    bookmarkStore.init();
    notifications.init();
    loadSessions();
    startAutoRefresh();
    setupKeyboardShortcuts();
    handleUrlParams();
    setupAgentEventListener();
    setupIngestProgressListener();
    handleWindowResize();

    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("mousemove", handleResize);
    window.addEventListener("mouseup", stopResizing);
    window.addEventListener("resize", handleWindowResize);

    return () => {
      stopAutoRefresh();
      if (unlistenAgentEvents) unlistenAgentEvents();
      if (unlistenIngestProgress) unlistenIngestProgress();
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("mousemove", handleResize);
      window.removeEventListener("mouseup", stopResizing);
      window.removeEventListener("resize", handleWindowResize);
    };
  });

  onDestroy(() => {
    stopAutoRefresh();
    if (unlistenAgentEvents) unlistenAgentEvents();
    if (unlistenIngestProgress) unlistenIngestProgress();
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
      if (hasDiffOnly) params.set("hasDiff", "1");
      if (errorsOnly) params.set("errors", "1");

      const url = params.toString() ? `?${params.toString()}` : "/";
      goto(url, { replaceState: true, keepFocus: true });
    }
  });

  $effect(() => {
    if (activeTab !== "sessions") {
      showSessionListDrawer = false;
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

<Sheet bind:open={bookmarksOpen} side="right" width="md" aria-label="Bookmarks">
  <div class="flex flex-col h-full">
    <div class="flex items-center justify-between p-4 border-b border-surface-muted">
      <h2 class="text-lg font-semibold text-fg m-0">Bookmarks</h2>
      <button
        class="p-2 text-fg-dim hover:text-fg transition-colors"
        onclick={() => (bookmarksOpen = false)}
        aria-label="Close bookmarks">
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
            class="group flex items-start gap-3 p-3 bg-surface-soft rounded border border-surface-muted hover:border-blue transition-colors"
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
</Sheet>

{#if showEventInspector && selectedEvent}
  <Modal bind:open={showEventInspector} size="lg" aria-label="Event inspector">
    <div class="h-[80vh]">
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
  </Modal>
{/if}

{#if showSessionMetaModal && selectedSession}
  <Modal bind:open={showSessionMetaModal} size="xl" contentClass="h-[85vh] flex flex-col" aria-label="Session details">
    <div class="flex items-center justify-between px-6 py-4 border-b border-surface-muted bg-surface-soft">
      <div class="flex items-center gap-3">
        <h2 class="text-xl font-semibold text-fg m-0">
          {selectedSession.title || "Untitled Session"}
        </h2>
        <span class="px-2 py-0.5 bg-surface-muted rounded text-2xs text-fg-dim uppercase">
          {selectedSession.source}
        </span>
      </div>
      <div class="flex items-center gap-2">
        <button
          class="px-3 py-1.5 bg-surface border border-surface-muted rounded text-sm text-fg hover:border-blue hover:text-blue transition-colors flex items-center gap-1"
          onclick={() => copyToClipboard(JSON.stringify(selectedSession, null, 2))}
          type="button">
          <span class="i-ri-file-copy-line"></span>
          Copy Session JSON
        </button>
        <button
          class="p-2 text-fg-dim hover:text-fg transition-colors"
          onclick={() => (showSessionMetaModal = false)}
          type="button"
          aria-label="Close session details">
          <span class="i-ri-close-line text-xl"></span>
        </button>
      </div>
    </div>
    <div class="flex-1 overflow-auto p-6">
      <div class="mb-6 grid grid-cols-3 gap-4 text-sm">
        <div class="p-3 bg-surface-soft rounded border border-surface-muted">
          <div class="text-xs text-fg-muted mb-1">Session ID</div>
          <div class="text-fg font-mono text-xs">{selectedSession.id}</div>
        </div>
        <div class="p-3 bg-surface-soft rounded border border-surface-muted">
          <div class="text-xs text-fg-muted mb-1">External ID</div>
          <div class="text-fg font-mono text-xs">{selectedSession.external_id}</div>
        </div>
        <div class="p-3 bg-surface-soft rounded border border-surface-muted">
          <div class="text-xs text-fg-muted mb-1">Project</div>
          <div class="text-fg">{selectedSession.project || "No project"}</div>
        </div>
        <div class="p-3 bg-surface-soft rounded border border-surface-muted">
          <div class="text-xs text-fg-muted mb-1">Created</div>
          <div class="text-fg">{new Date(selectedSession.created_at).toLocaleString()}</div>
        </div>
        <div class="p-3 bg-surface-soft rounded border border-surface-muted">
          <div class="text-xs text-fg-muted mb-1">Updated</div>
          <div class="text-fg">{new Date(selectedSession.updated_at).toLocaleString()}</div>
        </div>
        <div class="p-3 bg-surface-soft rounded border border-surface-muted">
          <div class="text-xs text-fg-muted mb-1">Events</div>
          <div class="text-fg">{events.length} events</div>
        </div>
      </div>
      <div class="bg-surface-soft rounded border border-surface-muted overflow-hidden">
        <div class="px-4 py-2 border-b border-surface-muted bg-surface-muted/50 flex items-center justify-between">
          <span class="text-sm font-semibold text-fg">Full Session Data</span>
          <span class="text-2xs text-fg-dim">JSON</span>
        </div>
        <pre class="p-4 text-sm text-fg-dim overflow-x-auto max-h-[50vh]"><code>{JSON.stringify(selectedSession, null, 2)}</code></pre>
      </div>
    </div>
  </Modal>
{/if}

{#if activeTab === "sessions" && isNarrowLayout}
  <Drawer bind:open={showSessionListDrawer} direction="left" size="lg" aria-label="Session list drawer">
    <div class="h-full flex flex-col bg-surface-soft">
      <div class="px-4 py-3 border-b border-surface-muted flex items-center justify-between">
        <h2 class="m-0 text-sm font-semibold text-fg">Sessions</h2>
        <span class="text-2xs text-fg-dim">{filteredSessions.length}</span>
      </div>
      <div class="flex-1 overflow-hidden">
        <SessionList sessions={filteredSessions} {selectedSession} onSelect={selectSession} />
      </div>
    </div>
  </Drawer>
{/if}

<div class="flex h-screen overflow-hidden flex-col">
  <header class="border-b border-surface-muted bg-surface">
    <div class="px-4 py-3 flex items-center gap-3 flex-wrap">
      <div class="flex items-center gap-2">
        {#if activeTab === "sessions" && isNarrowLayout}
          <button
            class="p-2 border border-surface-muted rounded bg-surface-soft text-fg-dim hover:text-fg"
            onclick={() => (showSessionListDrawer = true)}
            type="button"
            title="Open sessions list">
            <span class="i-ri-menu-line"></span>
          </button>
        {/if}
        <h1 class="m-0 text-lg font-semibold text-fg">Agent V</h1>
      </div>

      <nav class="flex items-center gap-1 rounded border border-surface-muted bg-surface-soft p-1">
        {#each tabs as tab}
          <button
            class="px-3 py-1.5 rounded text-xs transition-colors border border-transparent flex items-center gap-1.5 {activeTab ===
            tab.id
              ? 'bg-surface border-surface-muted text-blue'
              : 'bg-transparent text-fg-dim hover:text-fg'}"
            onclick={() => (activeTab = tab.id)}
            type="button">
            <span class={tab.icon}></span>
            <span>{tab.label}</span>
          </button>
        {/each}
      </nav>

      <div class="ml-auto flex items-center gap-2 flex-wrap">
        <button
          class="px-2.5 py-1.5 border rounded text-xs bg-surface-soft border-surface-muted text-fg-dim hover:text-fg flex items-center gap-1.5"
          onclick={toggleAutoRefresh}
          type="button">
          <span class={autoRefreshEnabled ? "i-ri-checkbox-circle-line text-green" : "i-ri-checkbox-blank-circle-line"}></span>
          <span>{autoRefreshEnabled ? "Auto" : "Manual"}</span>
        </button>

        <button
          class="px-2.5 py-1.5 border rounded text-xs bg-surface-soft border-surface-muted text-fg-dim hover:text-fg flex items-center gap-1.5"
          onclick={loadSessions}
          type="button">
          <span class="i-ri-refresh-line"></span>
          <span>Refresh</span>
        </button>

        <button
          class="px-2.5 py-1.5 border rounded text-xs bg-blue text-surface border-blue hover:bg-blue-bright disabled:opacity-50"
          onclick={ingestAllSources}
          disabled={ingestLoading}
          type="button">
          {ingestLoading ? "Ingesting..." : "Ingest All"}
        </button>

        <div class="hidden sm:flex items-center gap-1 rounded border border-surface-muted bg-surface-soft p-1">
          <span class="text-2xs text-fg-dim px-1">Export</span>
          <button
            class="px-2 py-1 rounded text-2xs text-fg-dim hover:text-fg"
            onclick={() => exportSelectedSession("md")}
            type="button">
            .md
          </button>
          <button
            class="px-2 py-1 rounded text-2xs text-fg-dim hover:text-fg"
            onclick={() => exportSelectedSession("json")}
            type="button">
            .json
          </button>
          <button
            class="px-2 py-1 rounded text-2xs text-fg-dim hover:text-fg"
            onclick={() => exportSelectedSession("jsonl")}
            type="button">
            .jsonl
          </button>
        </div>

        <button
          class="p-2 text-fg-dim hover:text-fg transition-colors relative"
          onclick={() => (bookmarksOpen = !bookmarksOpen)}
          title="Bookmarks (Cmd+B)">
          <span class="i-ri-bookmark-line"></span>
          {#if bookmarkStore.bookmarks.length > 0}
            <span class="absolute top-1 right-1 w-2 h-2 bg-blue rounded-full" aria-hidden="true"> </span>
          {/if}
        </button>

        <button
          class="px-2.5 py-1.5 border rounded text-xs bg-surface-soft border-surface-muted text-fg-dim hover:text-fg flex items-center gap-1.5"
          onclick={keyboardStore.openCommandPalette}
          type="button">
          <span class="i-ri-command-line"></span>
          <span>Cmd+K</span>
        </button>
      </div>
    </div>

    <div class="px-4 py-2 border-t border-surface-muted bg-surface-soft flex items-center gap-2 flex-wrap">
      <span class="text-2xs text-fg-dim uppercase tracking-wide">Sources</span>
      <div class="flex items-center gap-1 rounded border border-surface-muted p-1 bg-surface">
        <button
          class="px-2 py-1 rounded text-2xs border border-transparent {filterStore.state.source === null
            ? 'bg-surface-soft border-surface-muted text-blue'
            : 'text-fg-dim hover:text-fg'}"
          onclick={() => setSourceScope(null)}
          type="button">
          All
        </button>
        {#each sources as source}
          <button
            class="px-2 py-1 rounded text-2xs border border-transparent {filterStore.state.source === source.id
              ? 'bg-surface-soft border-surface-muted text-blue'
              : 'text-fg-dim hover:text-fg'}"
            onclick={() => setSourceScope(source.id)}
            type="button">
            {source.name}
          </button>
        {/each}
      </div>

      <button
        class="px-2.5 py-1.5 border rounded text-xs bg-surface border-surface-muted text-fg-dim hover:text-fg flex items-center gap-1"
        onclick={() => (showTopFilters = !showTopFilters)}
        type="button">
        <span class="i-ri-filter-3-line"></span>
        <span>{showTopFilters ? "Hide Filters" : "Filters"}</span>
      </button>

      {#if filterStore.state.source}
        <button
          class="px-2.5 py-1.5 border rounded text-xs bg-surface border-surface-muted text-fg-dim hover:text-fg"
          onclick={() => ingestSource(filterStore.state.source!)}
          disabled={ingestLoading}
          type="button">
          Refresh {filterStore.state.source}
        </button>
      {/if}

      {#if newSessionsAvailable}
        <div class="ml-auto px-3 py-1.5 bg-yellow/20 border border-yellow rounded text-xs text-yellow flex items-center gap-2" transition:slide>
          <span class="i-ri-notification-3-line animate-pulse"></span>
          <span>New sessions available</span>
          <button
            class="bg-transparent border-none p-0 text-yellow font-semibold cursor-pointer hover:underline"
            onclick={loadSessions}
            type="button">
            Load
          </button>
        </div>
      {/if}
    </div>

    {#if showTopFilters}
      <div class="px-4 py-3 border-t border-surface-muted bg-surface" transition:slide>
        <div class="grid gap-3 lg:grid-cols-[minmax(0,2fr)_180px_auto_auto_auto] items-center">
          <div class="relative">
            <input
              type="text"
              class="w-full px-3 py-2 pl-9 bg-surface-soft border border-surface-muted rounded text-fg text-sm focus:outline-none focus:border-blue"
              placeholder="Search sessions..."
              bind:value={filterStore.state.query} />
            <span class="absolute left-3 top-1/2 -translate-y-1/2 i-ri-search-line text-fg-muted"></span>
          </div>

          <label class="flex items-center gap-2 text-xs text-fg-dim">
            <span>Date</span>
            <select
              class="px-2 py-2 bg-surface-soft border border-surface-muted rounded text-sm text-fg"
              value={filterStore.state.since || ""}
              onchange={(e) => filterStore.setFilter("since", e.currentTarget.value || null)}>
              {#each dateRanges as range}
                <option value={range.value}>{range.label}</option>
              {/each}
            </select>
          </label>

          <button
            class="px-3 py-2 border rounded text-xs transition-colors {hasDiffOnly
              ? 'bg-blue/15 border-blue text-blue'
              : 'bg-surface-soft border-surface-muted text-fg-dim hover:text-fg'}"
            onclick={() => (hasDiffOnly = !hasDiffOnly)}
            type="button">
            Has diff
          </button>

          <button
            class="px-3 py-2 border rounded text-xs transition-colors {errorsOnly
              ? 'bg-blue/15 border-blue text-blue'
              : 'bg-surface-soft border-surface-muted text-fg-dim hover:text-fg'}"
            onclick={() => (errorsOnly = !errorsOnly)}
            type="button">
            Errors
          </button>

          <div class="flex items-center gap-2 justify-end">
            <button
              class="px-3 py-2 bg-surface-soft border border-surface-muted rounded text-xs text-fg-dim hover:text-fg"
              onclick={() => (activeTab = "search")}
              type="button">
              Open Search
            </button>
            <button
              class="px-3 py-2 bg-transparent border border-surface-muted rounded text-xs text-fg-dim hover:text-fg"
              onclick={clearTopFilters}
              type="button">
              Clear
            </button>
          </div>
        </div>
      </div>
    {/if}

    {#if error}
      <div class="mx-4 my-2 p-2 bg-red text-surface rounded text-xs" transition:fade>
        {error}
      </div>
    {/if}
  </header>

  <div class="flex-1 overflow-hidden">
    {#if activeTab === "sessions"}
      <div class="flex h-full overflow-hidden">
        {#if !isNarrowLayout}
          <aside
            class="bg-surface-soft border-r border-surface-muted flex flex-col overflow-hidden relative"
            style="width: {sidebarWidth}px; min-width: {minSidebarWidth}px;">
            <!-- TODO: address these -->
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <div
              class="absolute top-0 right-0 w-1 h-full cursor-col-resize hover:bg-blue/50 transition-colors z-10"
              onmousedown={startResizing}
              onkeydown={handleResizerKeydown}
              role="separator"
              aria-label="Resize sidebar"
              aria-valuenow={sidebarWidth}
              aria-valuemin={minSidebarWidth}
              aria-valuemax={maxSidebarWidth}
              tabindex="0">
            </div>

            <div class="flex-1 overflow-hidden">
              <SessionList sessions={filteredSessions} {selectedSession} onSelect={selectSession} />
            </div>

            <div class="p-2 border-t border-surface-muted bg-surface text-xs text-fg-dim">
              <div class="flex justify-between items-center">
                <span>{filteredSessions.length} shown</span>
                <span>{sessions.length} total</span>
              </div>
              {#if lastIngestTime}
                <div class="mt-1">Last update: {lastIngestTime.toLocaleTimeString()}</div>
              {/if}
            </div>
          </aside>
        {/if}

        <main class="flex-1 overflow-hidden flex flex-col">
          {#if sessions.length === 0 && !loading}
            <WelcomeScreen onGetStarted={ingestAllSources} />
          {:else if selectedSession}
            <SessionViewer
              session={selectedSession}
              {events}
              onSelectEvent={selectEvent}
              onOpenDrawer={() => (showSessionMetaModal = true)} />
          {:else}
            <div class="flex-1 flex items-center justify-center text-fg-dim px-6" in:fade>
              <div class="text-center max-w-lg">
                <div class="i-ri-chat-3-line text-4xl mb-3 opacity-50"></div>
                <p class="m-0 text-base text-fg">Select a session</p>
                <p class="text-sm text-fg-muted mt-2 mb-4">Choose a session from the left pane to inspect timeline details.</p>
                <div class="text-sm text-fg-dim space-y-1">
                  <p class="m-0">Press Cmd+K to jump to a command or session.</p>
                  <p class="m-0">Use the top search and filter chips to narrow results.</p>
                  <p class="m-0">Use Live after opening a session to follow new events.</p>
                </div>
                <div class="mt-4 flex items-center justify-center gap-2 flex-wrap">
                  <button
                    class="px-3 py-2 bg-surface-soft border border-surface-muted rounded text-xs text-fg-dim hover:text-fg"
                    onclick={keyboardStore.openCommandPalette}
                    type="button">
                    Cmd+K Command Palette
                  </button>
                  <button
                    class="px-3 py-2 bg-surface-soft border border-surface-muted rounded text-xs text-fg-dim hover:text-fg"
                    onclick={followLatestSession}
                    type="button">
                    Open Latest Session
                  </button>
                </div>
              </div>
            </div>
          {/if}
        </main>
      </div>
    {:else}
      <main class="h-full overflow-hidden">
        {#if activeTab === "search"}
          <SearchPanel onSelectSession={selectSessionById} onSelectEvent={selectEvent} />
        {:else if activeTab === "analytics"}
          <AnalyticsPanel />
        {:else if activeTab === "support"}
          <SupportPanel />
        {:else}
          <IngestStatusPanel onRefresh={loadSessions} />
        {/if}
      </main>
    {/if}
  </div>
</div>

<Dialog bind:open={showSupportNudge} closeOnOutsideClick={false} role="dialog" aria-label="Support Agent V">
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center p-4">
    <div class="bg-surface border border-surface-muted rounded-lg max-w-md w-full shadow-xl">
      <div class="p-6">
        <div class="flex items-center gap-3 mb-4">
          <span class="i-ri-heart-line text-3xl text-red"></span>
          <h2 class="m-0 text-xl font-semibold text-fg">Support Agent V</h2>
        </div>

        <p class="m-0 mb-4 text-fg-dim leading-relaxed">
          Great! You've successfully imported your first sessions. Agent V is <strong>donationware</strong> - free to use
          with no paid tiers.
        </p>

        <p class="m-0 mb-6 text-fg-dim leading-relaxed">
          If Agent V helps your workflow, consider supporting its continued development. Your contribution keeps the
          project independent and privacy-focused.
        </p>

        <div class="space-y-3 mb-6">
          <a
            href="https://github.com/sponsors/desertthunder"
            target="_blank"
            rel="noopener noreferrer"
            class="flex items-center gap-3 p-3 bg-surface-soft border border-surface-muted rounded-lg hover:border-blue transition-all no-underline">
            <span class="i-ri-github-line text-xl text-fg-dim"></span>
            <div class="flex-1">
              <div class="font-medium text-fg">GitHub Sponsors</div>
              <div class="text-sm text-fg-dim">Monthly or one-time support</div>
            </div>
            <span class="i-ri-external-link-line text-fg-dim"></span>
          </a>

          <a
            href="https://ko-fi.com/desertthunder"
            target="_blank"
            rel="noopener noreferrer"
            class="flex items-center gap-3 p-3 bg-surface-soft border border-surface-muted rounded-lg hover:border-blue transition-all no-underline">
            <span class="i-ri-cup-line text-xl text-fg-dim"></span>
            <div class="flex-1">
              <div class="font-medium text-fg">Ko-fi</div>
              <div class="text-sm text-fg-dim">Quick one-time donations</div>
            </div>
            <span class="i-ri-external-link-line text-fg-dim"></span>
          </a>
        </div>

        <div class="flex gap-3">
          <button
            class="flex-1 px-4 py-2 bg-blue text-surface border-none rounded font-medium cursor-pointer hover:bg-blue-bright transition-colors"
            onclick={() => {
              showSupportNudge = false;
              activeTab = "support";
            }}>
            Learn More
          </button>
          <button
            class="px-4 py-2 bg-transparent border border-surface-muted rounded text-fg cursor-pointer hover:border-fg transition-colors"
            onclick={() => {
              supportNudgeStore.dismissNudge();
              showSupportNudge = false;
            }}>
            Dismiss
          </button>
        </div>
      </div>
    </div>
  </div>
</Dialog>

{#if ingestProgress}
  <div class="fixed bottom-0 left-0 right-0 z-50" transition:fade={{ duration: 200 }}>
    <div
      class="flex items-center justify-between px-3 py-1 bg-surface-soft/90 backdrop-blur-sm border-t border-surface-muted text-xs text-fg-dim">
      <span>
        Ingesting {ingestProgress.source}... {ingestProgress.current}/{ingestProgress.total}
      </span>
      <span>{ingestProgress.total > 0 ? Math.round((ingestProgress.current / ingestProgress.total) * 100) : 0}%</span>
    </div>
    <div class="h-1 bg-surface-muted">
      <div
        class="h-full bg-blue transition-all duration-300 ease-out"
        style="width: {ingestProgress.total > 0 ? (ingestProgress.current / ingestProgress.total) * 100 : 0}%">
      </div>
    </div>
  </div>
{/if}
