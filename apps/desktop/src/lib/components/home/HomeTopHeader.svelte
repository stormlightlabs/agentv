<script lang="ts">
  import { bookmarkStore } from "$lib/stores/bookmarks.svelte";
  import { filterStore } from "$lib/stores/filters.svelte";
  import { keyboardStore } from "$lib/stores/keyboard.svelte";
  import type { ExportFormat } from "$lib/types";
  import { fade, slide } from "svelte/transition";

  type TabId = "sessions" | "search" | "analytics" | "status" | "support";

  type Props = {
    activeTab: TabId;
    bookmarksOpen: boolean;
    isNarrowLayout: boolean;
    autoRefreshEnabled: boolean;
    ingestLoading: boolean;
    showTopFilters: boolean;
    hasDiffOnly: boolean;
    errorsOnly: boolean;
    newSessionsAvailable: boolean;
    error: string | null;
    onOpenSessionList?: () => void;
    onToggleAutoRefresh?: () => void;
    onRefreshSessions?: () => void;
    onIngestAllSources?: () => void;
    onExportSession?: (format: ExportFormat) => void;
    onIngestSource?: (source: string) => void;
    onLoadNewSessions?: () => void;
  };

  let {
    activeTab = $bindable<TabId>(),
    bookmarksOpen = $bindable(false),
    isNarrowLayout,
    autoRefreshEnabled,
    ingestLoading,
    showTopFilters = $bindable(false),
    hasDiffOnly = $bindable(false),
    errorsOnly = $bindable(false),
    newSessionsAvailable,
    error,
    onOpenSessionList,
    onToggleAutoRefresh,
    onRefreshSessions,
    onIngestAllSources,
    onExportSession,
    onIngestSource,
    onLoadNewSessions,
  }: Props = $props();

  const tabs: Array<{ id: TabId; label: string; icon: string }> = [
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

  function setSourceScope(source: string | null) {
    filterStore.setFilter("source", source);
  }

  function clearTopFilters() {
    filterStore.setFilter("query", "");
    filterStore.setFilter("since", null);
    hasDiffOnly = false;
    errorsOnly = false;
  }
</script>

<header class="border-surface-muted bg-surface border-b">
  <div class="flex flex-wrap items-center gap-3 px-4 py-3">
    <div class="flex items-center gap-2">
      {#if activeTab === "sessions" && isNarrowLayout}
        <button
          class="border-surface-muted bg-surface-soft text-fg-dim hover:text-fg rounded border p-2"
          onclick={onOpenSessionList}
          type="button"
          title="Open sessions list">
          <span class="i-ri-menu-line"></span>
        </button>
      {/if}
      <h1 class="text-fg m-0 text-lg font-semibold">Agent V</h1>
    </div>

    <nav class="border-surface-muted bg-surface-soft flex items-center gap-1 rounded border p-1">
      {#each tabs as tab (tab.id)}
        <button
          class="flex items-center gap-1.5 rounded border border-transparent px-3 py-1.5 text-xs transition-colors {activeTab ===
          tab.id
            ? 'bg-surface border-surface-muted text-blue'
            : 'text-fg-dim hover:text-fg bg-transparent'}"
          onclick={() => (activeTab = tab.id)}
          type="button">
          <span class={tab.icon}></span>
          <span>{tab.label}</span>
        </button>
      {/each}
    </nav>

    <div class="ml-auto flex flex-wrap items-center gap-2">
      <button
        class="bg-surface-soft border-surface-muted text-fg-dim hover:text-fg flex items-center gap-1.5 rounded border px-2.5 py-1.5 text-xs"
        onclick={onToggleAutoRefresh}
        type="button">
        <span class={autoRefreshEnabled ? "i-ri-checkbox-circle-line text-green" : "i-ri-checkbox-blank-circle-line"}></span>
        <span>{autoRefreshEnabled ? "Auto" : "Manual"}</span>
      </button>

      <button
        class="bg-surface-soft border-surface-muted text-fg-dim hover:text-fg flex items-center gap-1.5 rounded border px-2.5 py-1.5 text-xs"
        onclick={onRefreshSessions}
        type="button">
        <span class="i-ri-refresh-line"></span>
        <span>Refresh</span>
      </button>

      <button
        class="bg-blue text-surface border-blue hover:bg-blue-bright rounded border px-2.5 py-1.5 text-xs disabled:opacity-50"
        onclick={onIngestAllSources}
        disabled={ingestLoading}
        type="button">
        {ingestLoading ? "Ingesting..." : "Ingest All"}
      </button>

      <div class="border-surface-muted bg-surface-soft hidden items-center gap-1 rounded border p-1 sm:flex">
        <span class="text-2xs text-fg-dim px-1">Export</span>
        <button
          class="text-2xs text-fg-dim hover:text-fg rounded px-2 py-1"
          onclick={() => onExportSession?.("md")}
          type="button">
          .md
        </button>
        <button
          class="text-2xs text-fg-dim hover:text-fg rounded px-2 py-1"
          onclick={() => onExportSession?.("json")}
          type="button">
          .json
        </button>
        <button
          class="text-2xs text-fg-dim hover:text-fg rounded px-2 py-1"
          onclick={() => onExportSession?.("jsonl")}
          type="button">
          .jsonl
        </button>
      </div>

      <button
        class="text-fg-dim hover:text-fg relative p-2 transition-colors"
        onclick={() => (bookmarksOpen = !bookmarksOpen)}
        title="Bookmarks (Cmd+B)"
        type="button">
        <span class="i-ri-bookmark-line"></span>
        {#if bookmarkStore.bookmarks.length > 0}
          <span class="bg-blue absolute top-1 right-1 h-2 w-2 rounded-full" aria-hidden="true"> </span>
        {/if}
      </button>

      <button
        class="bg-surface-soft border-surface-muted text-fg-dim hover:text-fg flex items-center gap-1.5 rounded border px-2.5 py-1.5 text-xs"
        onclick={keyboardStore.openCommandPalette}
        type="button">
        <span class="i-ri-command-line"></span>
        <span>Cmd+K</span>
      </button>
    </div>
  </div>

  <div class="border-surface-muted bg-surface-soft flex flex-wrap items-center gap-2 border-t px-4 py-2">
    <span class="text-2xs text-fg-dim tracking-wide uppercase">Sources</span>
    <div class="border-surface-muted bg-surface flex items-center gap-1 rounded border p-1">
      <button
        class="text-2xs rounded border border-transparent px-2 py-1 {filterStore.state.source === null
          ? 'bg-surface-soft border-surface-muted text-blue'
          : 'text-fg-dim hover:text-fg'}"
        onclick={() => setSourceScope(null)}
        type="button">
        All
      </button>
      {#each sources as source (source.id)}
        <button
          class="text-2xs rounded border border-transparent px-2 py-1 {filterStore.state.source === source.id
            ? 'bg-surface-soft border-surface-muted text-blue'
            : 'text-fg-dim hover:text-fg'}"
          onclick={() => setSourceScope(source.id)}
          type="button">
          {source.name}
        </button>
      {/each}
    </div>

    <button
      class="bg-surface border-surface-muted text-fg-dim hover:text-fg flex items-center gap-1 rounded border px-2.5 py-1.5 text-xs"
      onclick={() => (showTopFilters = !showTopFilters)}
      type="button">
      <span class="i-ri-filter-3-line"></span>
      <span>{showTopFilters ? "Hide Filters" : "Filters"}</span>
    </button>

    {#if filterStore.state.source}
      <button
        class="bg-surface border-surface-muted text-fg-dim hover:text-fg rounded border px-2.5 py-1.5 text-xs"
        onclick={() => onIngestSource?.(filterStore.state.source!)}
        disabled={ingestLoading}
        type="button">
        Refresh {filterStore.state.source}
      </button>
    {/if}

    {#if newSessionsAvailable}
      <div
        class="bg-yellow/20 border-yellow text-yellow ml-auto flex items-center gap-2 rounded border px-3 py-1.5 text-xs"
        transition:slide>
        <span class="i-ri-notification-3-line animate-pulse"></span>
        <span>New sessions available</span>
        <button
          class="text-yellow cursor-pointer border-none bg-transparent p-0 font-semibold hover:underline"
          onclick={onLoadNewSessions}
          type="button">
          Load
        </button>
      </div>
    {/if}
  </div>

  {#if showTopFilters}
    <div class="border-surface-muted bg-surface border-t px-4 py-3" transition:slide>
      <div class="grid items-center gap-3 lg:grid-cols-[minmax(0,2fr)_180px_auto_auto_auto]">
        <div class="relative">
          <input
            type="text"
            class="bg-surface-soft border-surface-muted text-fg focus:border-blue w-full rounded border px-3 py-2 pl-9 text-sm focus:outline-none"
            placeholder="Search sessions..."
            bind:value={filterStore.state.query} />
          <span class="i-ri-search-line text-fg-muted absolute top-1/2 left-3 -translate-y-1/2"></span>
        </div>

        <label class="text-fg-dim flex items-center gap-2 text-xs">
          <span>Date</span>
          <select
            class="bg-surface-soft border-surface-muted text-fg rounded border px-2 py-2 text-sm"
            value={filterStore.state.since || ""}
            onchange={(event) => filterStore.setFilter("since", event.currentTarget.value || null)}>
            {#each dateRanges as range (range.value)}
              <option value={range.value}>{range.label}</option>
            {/each}
          </select>
        </label>

        <button
          class="rounded border px-3 py-2 text-xs transition-colors {hasDiffOnly
            ? 'bg-blue/15 border-blue text-blue'
            : 'bg-surface-soft border-surface-muted text-fg-dim hover:text-fg'}"
          onclick={() => (hasDiffOnly = !hasDiffOnly)}
          type="button">
          Has diff
        </button>

        <button
          class="rounded border px-3 py-2 text-xs transition-colors {errorsOnly
            ? 'bg-blue/15 border-blue text-blue'
            : 'bg-surface-soft border-surface-muted text-fg-dim hover:text-fg'}"
          onclick={() => (errorsOnly = !errorsOnly)}
          type="button">
          Errors
        </button>

        <div class="flex items-center justify-end gap-2">
          <button
            class="bg-surface-soft border-surface-muted text-fg-dim hover:text-fg rounded border px-3 py-2 text-xs"
            onclick={() => (activeTab = "search")}
            type="button">
            Open Search
          </button>
          <button
            class="border-surface-muted text-fg-dim hover:text-fg rounded border bg-transparent px-3 py-2 text-xs"
            onclick={clearTopFilters}
            type="button">
            Clear
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if error}
    <div class="bg-red text-surface mx-4 my-2 rounded p-2 text-xs" transition:fade>
      {error}
    </div>
  {/if}
</header>
