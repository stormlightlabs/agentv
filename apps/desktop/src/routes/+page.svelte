<script lang="ts">
  import { browser } from "$app/environment";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import AnalyticsPanel from "$lib/components/AnalyticsPanel.svelte";
  import CommandPalette from "$lib/components/CommandPalette.svelte";
  import HomeBookmarksSheet from "$lib/components/home/HomeBookmarksSheet.svelte";
  import HomeEventInspectorModal from "$lib/components/home/HomeEventInspectorModal.svelte";
  import HomeIngestProgressFooter from "$lib/components/home/HomeIngestProgressFooter.svelte";
  import HomeSessionListDrawer from "$lib/components/home/HomeSessionListDrawer.svelte";
  import HomeSessionMetaModal from "$lib/components/home/HomeSessionMetaModal.svelte";
  import HomeSessionsTab from "$lib/components/home/HomeSessionsTab.svelte";
  import HomeSupportDialog from "$lib/components/home/HomeSupportDialog.svelte";
  import HomeTopHeader from "$lib/components/home/HomeTopHeader.svelte";
  import IngestStatusPanel from "$lib/components/IngestStatusPanel.svelte";
  import SearchPanel from "$lib/components/SearchPanel.svelte";
  import SupportPanel from "$lib/components/SupportPanel.svelte";
  import Toast from "$lib/components/Toast.svelte";
  import {
    bookmarkStore,
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
  import { resolve } from "$app/paths";
  import { SvelteURLSearchParams } from "svelte/reactivity";

  type Tab = "sessions" | "search" | "analytics" | "status" | "support";

  const toast = useToast();
  const notifications = useNotifications();

  const minSidebarWidth = 350;
  const maxSidebarWidth = 800;
  const minSessionViewerWidth = 520;
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

  const params = $derived.by(() => {
    const p = new SvelteURLSearchParams();
    if (activeTab !== "sessions") p.set("tab", activeTab);
    if (filterStore.state.query) p.set("q", filterStore.state.query);
    if (filterStore.state.sessionId) p.set("session", filterStore.state.sessionId);
    if (filterStore.state.source) p.set("source", filterStore.state.source);
    if (filterStore.state.project) p.set("project", filterStore.state.project);
    if (filterStore.state.kind) p.set("kind", filterStore.state.kind);
    if (filterStore.state.role) p.set("role", filterStore.state.role);
    if (filterStore.state.since) p.set("since", filterStore.state.since);
    if (hasDiffOnly) p.set("hasDiff", "1");
    if (errorsOnly) p.set("errors", "1");

    return p;
  });

  let refreshInterval: ReturnType<typeof setInterval> | null = null;
  let showSupportNudge = $state(false);
  let ingestProgress = $state<IngestProgress | null>(null);
  let progressHideTimeout: ReturnType<typeof setTimeout> | null = null;
  const refreshProgressLabel = "sessions";
  const ingestAllProgressLabel = "all sources";

  function clearProgressHideTimeout() {
    if (progressHideTimeout) {
      clearTimeout(progressHideTimeout);
      progressHideTimeout = null;
    }
  }

  function setIngestProgress(progress: IngestProgress) {
    clearProgressHideTimeout();
    ingestProgress = progress;
  }

  function completeIngestProgress(source: string, total: number, phase = "Complete") {
    clearProgressHideTimeout();
    ingestProgress = { source, phase, current: total, total };
    progressHideTimeout = globalThis.setTimeout(() => {
      ingestProgress = null;
      progressHideTimeout = null;
    }, 1500);
  }

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
    return [...filteredSessions].toSorted((a, b) => b.updated_at.localeCompare(a.updated_at))[0] ?? null;
  });

  async function loadSessionMetrics() {
    try {
      const metrics = await invoke<SessionListMetricsData[]>("list_session_metrics", {
        limit: Math.max(2000, sessions.length + 100),
        offset: 0,
      });
      sessionMetricsById = Object.fromEntries(metrics.map((metric) => [metric.session_id, metric]));
    } catch (error_) {
      console.error("Failed to load session metrics:", error_);
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
    } catch (error_) {
      error = String(error_);
      toast.error(`Failed to load sessions: ${error_}`);
    } finally {
      loading = false;
    }
  }

  async function refreshSessionsWithProgress(source = refreshProgressLabel) {
    setIngestProgress({ source, phase: "Refreshing", current: 0, total: 1 });
    try {
      await loadSessions();
    } finally {
      completeIngestProgress(source, 1);
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
    } catch (error_) {
      console.error("Failed to check for new sessions:", error_);
    }
  }

  function startAutoRefresh() {
    if (refreshInterval) return;

    refreshInterval = globalThis.setInterval(() => {
      checkForNewSessions();
    }, 120_000);
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

  function handleWindowResize() {
    isNarrowLayout = window.innerWidth < narrowLayoutBreakpoint;
    const sidebarMaxForViewport = Math.max(minSidebarWidth, Math.min(maxSidebarWidth, window.innerWidth - minSessionViewerWidth));
    sidebarWidth = Math.min(sidebarWidth, sidebarMaxForViewport);
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
      document.body.append(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      toast.success(`Exported ${format.toUpperCase()}`);
    } catch (error_) {
      toast.error(`Failed to export session: ${error_}`);
    }
  }

  async function selectSession(session: SessionData) {
    selectedSession = session;
    filterStore.setFilter("sessionId", session.id);
    showSessionListDrawer = false;
    try {
      events = await invoke<EventData[]>("get_session_events", { sessionId: session.id });
      logInfo("Session selected", { sessionId: session.id, eventCount: events.length });
    } catch (error_) {
      console.error("Failed to load events:", error_);
      toast.error(`Failed to load events: ${error_}`);
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
    const sidebarMaxForViewport = Math.max(minSidebarWidth, Math.min(maxSidebarWidth, window.innerWidth - minSessionViewerWidth));
    if (newWidth >= minSidebarWidth && newWidth <= sidebarMaxForViewport) {
      sidebarWidth = newWidth;
    }
  }

  function stopResizing() {
    isResizing = false;
  }

  function handleResizerKeydown(event: KeyboardEvent) {
    if (isNarrowLayout) return;

    const step = 20;
    switch (event.key) {
      case "ArrowLeft": {
        event.preventDefault();
        sidebarWidth = Math.max(minSidebarWidth, sidebarWidth - step);

        break;
      }
      case "ArrowRight": {
        event.preventDefault();
        sidebarWidth = Math.min(maxSidebarWidth, sidebarWidth + step);

        break;
      }
      case "Home": {
        event.preventDefault();
        sidebarWidth = minSidebarWidth;

        break;
      }
      case "End": {
        event.preventDefault();
        sidebarWidth = maxSidebarWidth;

        break;
      }
      // No default
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
    setIngestProgress({ source: sourceId, phase: "Ingesting", current: 0, total: 2 });

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

      setIngestProgress({ source: sourceId, phase: "Refreshing", current: 1, total: 2 });
      await loadSessions();
      completeIngestProgress(sourceId, 2);
    } catch (error_) {
      error = String(error_);
      toast.error(`Failed to ingest ${sourceId}: ${error_}`);
      completeIngestProgress(sourceId, 1, "Failed");
    } finally {
      ingestLoading = false;
    }
  }

  async function ingestAllSources() {
    ingestLoading = true;
    error = null;
    setIngestProgress({ source: ingestAllProgressLabel, phase: "Ingesting", current: 0, total: 2 });

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

      setIngestProgress({ source: ingestAllProgressLabel, phase: "Refreshing", current: 1, total: 2 });
      await loadSessions();
      completeIngestProgress(ingestAllProgressLabel, 2);
    } catch (error_) {
      error = String(error_);
      toast.error(`Failed to ingest all sources: ${error_}`);
      completeIngestProgress(ingestAllProgressLabel, 1, "Failed");
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
      case "session": {
        if (bookmark.data.sessionId) {
          selectSessionById(bookmark.data.sessionId);
        }
        break;
      }
      case "filter": {
        if (bookmark.data.filters) {
          for (const [key, value] of Object.entries(bookmark.data.filters)) {
            if (value) filterStore.setFilter(key as keyof typeof filterStore.state, value);
          }
        }
        break;
      }
      case "search": {
        if (bookmark.data.query) {
          filterStore.setFilter("query", bookmark.data.query);
          activeTab = "search";
        }
        break;
      }
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
      handler: refreshSessionsWithProgress,
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
        action: refreshSessionsWithProgress,
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
        completeIngestProgress(p.source, Math.max(p.total, 1));
      } else {
        setIngestProgress(p);
      }
    });
  }

  async function setupAgentEventListener() {
    unlistenAgentEvents = await listen<StreamingEventPayload>("agent-events", (event) => {
      const payload = event.payload;
      const isCurrentSession = selectedSession?.external_id === payload.session_external_id;

      if (payload.is_new_session) {
        loadSessions();
        notifications.notify(`New ${payload.source} session`, `New session from ${payload.source}`);
      } else if (isCurrentSession) {
        events = [...events, ...payload.events];
      }

      // Avoid spamming toasts for the exact session the user is actively viewing.
      const shouldNotifyEvent = !payload.is_new_session && payload.events.length > 0 && (!isCurrentSession || !notifications.windowFocused);
      if (shouldNotifyEvent) {
        const lastEvent = payload.events.at(-1)!;
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

    globalThis.addEventListener("keydown", handleKeydown);
    globalThis.addEventListener("mousemove", handleResize);
    globalThis.addEventListener("mouseup", stopResizing);
    window.addEventListener("resize", handleWindowResize);

    return () => {
      stopAutoRefresh();
      if (unlistenAgentEvents) unlistenAgentEvents();
      if (unlistenIngestProgress) unlistenIngestProgress();
      globalThis.removeEventListener("keydown", handleKeydown);
      globalThis.removeEventListener("mousemove", handleResize);
      globalThis.removeEventListener("mouseup", stopResizing);
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
      const url = resolve("/", Object.fromEntries(params));
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

<HomeBookmarksSheet
  open={bookmarksOpen}
  onOpenChange={(open) => (bookmarksOpen = open)}
  bookmarks={bookmarkStore.bookmarks}
  onApplyBookmark={applyBookmark}
  onDeleteBookmark={deleteBookmark} />

<HomeEventInspectorModal
  open={showEventInspector}
  onOpenChange={(open) => (showEventInspector = open)}
  {selectedEvent} />

<HomeSessionMetaModal
  open={showSessionMetaModal}
  onOpenChange={(open) => (showSessionMetaModal = open)}
  {selectedSession}
  eventsCount={events.length} />

{#if activeTab === "sessions" && isNarrowLayout}
  <HomeSessionListDrawer
    open={showSessionListDrawer}
    onOpenChange={(open) => (showSessionListDrawer = open)}
    {filteredSessions}
    {selectedSession}
    onSelectSession={selectSession} />
{/if}

<div class="flex h-screen flex-col overflow-hidden">
  <HomeTopHeader
    bind:activeTab
    bind:bookmarksOpen
    bind:showTopFilters
    bind:hasDiffOnly
    bind:errorsOnly
    {isNarrowLayout}
    {autoRefreshEnabled}
    {ingestLoading}
    {newSessionsAvailable}
    {error}
    onOpenSessionList={() => (showSessionListDrawer = true)}
    onToggleAutoRefresh={toggleAutoRefresh}
    onRefreshSessions={refreshSessionsWithProgress}
    onIngestAllSources={ingestAllSources}
    onExportSession={exportSelectedSession}
    onIngestSource={ingestSource}
    onLoadNewSessions={refreshSessionsWithProgress} />

  <div class="flex-1 overflow-hidden">
    {#if activeTab === "sessions"}
      <HomeSessionsTab
        state={{
          isNarrowLayout,
          sidebarWidth,
          filteredSessions,
          sessions,
          selectedSession,
          events,
          lastIngestTime,
          loading,
        }}
        actions={{
          onStartResizing: startResizing,
          onResizerKeydown: handleResizerKeydown,
          onSelectSession: selectSession,
          onGetStarted: ingestAllSources,
          onSelectEvent: selectEvent,
          onOpenSessionMeta: () => (showSessionMetaModal = true),
          onFollowLatestSession: followLatestSession,
        }} />
    {:else}
      <main class="h-full overflow-hidden">
        {#if activeTab === "search"}
          <SearchPanel onSelectSession={selectSessionById} onSelectEvent={selectEvent} />
        {:else if activeTab === "analytics"}
          <AnalyticsPanel />
        {:else if activeTab === "support"}
          <SupportPanel />
        {:else}
          <IngestStatusPanel onRefresh={refreshSessionsWithProgress} />
        {/if}
      </main>
    {/if}
  </div>
</div>

<HomeSupportDialog
  open={showSupportNudge}
  onOpenChange={(open) => (showSupportNudge = open)}
  onLearnMore={() => (activeTab = "support")}
  onDismiss={supportNudgeStore.dismissNudge} />

<HomeIngestProgressFooter {ingestProgress} />
