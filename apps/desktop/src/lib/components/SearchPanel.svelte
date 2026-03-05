<script lang="ts">
  import type { ActivityStats, SearchFacets, SearchResult } from "$lib/types";
  import { getDisplayProject } from "$lib/utils/sessionDisplay";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  import type { EventData } from "$lib/types";

  type Props = { onSelectSession?: (sessionId: string) => void; onSelectEvent?: (event: EventData) => void };

  let { onSelectSession, onSelectEvent }: Props = $props();

  let query = $state("");
  let results = $state<SearchResult[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  let facets: SearchFacets = $state({});
  let availableSources = $state<string[]>([]);
  let availableProjects = $state<string[]>([]);
  let availableKinds = $state<string[]>([]);

  let showFilters = $state(false);
  let activityStats = $state<ActivityStats[]>([]);
  let showAnalytics = $state(false);

  async function loadFacets() {
    try {
      const [sources, projects, kinds] = await Promise.all([
        invoke<string[]>("get_sources"),
        invoke<string[]>("get_projects"),
        invoke<string[]>("get_event_kinds"),
      ]);
      availableSources = sources;
      availableProjects = projects;
      availableKinds = kinds;
    } catch (error_) {
      console.error("Failed to load facets:", error_);
    }
  }

  async function performSearch() {
    if (!query.trim()) {
      results = [];
      return;
    }

    loading = true;
    error = null;

    try {
      const searchResults: SearchResult[] = await invoke("search_events", { query: query.trim(), facets, limit: 50 });
      results = searchResults;
    } catch (error_) {
      error = String(error_);
      results = [];
    } finally {
      loading = false;
    }
  }

  async function loadActivityStats() {
    try {
      const stats: ActivityStats[] = await invoke("get_activity_stats", { since: "30d", until: null });
      activityStats = stats;
    } catch (error_) {
      console.error("Failed to load activity stats:", error_);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      performSearch();
    }
  }

  function clearFilters() {
    facets = {};
    performSearch();
  }

  function formatTimestamp(ts: string): string {
    const date = new Date(ts);
    return date.toLocaleDateString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }

  function getKindLabel(kind: string): string {
    const labels: Record<string, string> = {
      message: "MSG",
      tool_call: "TOOL",
      tool_result: "RES",
      error: "ERR",
      system: "SYS",
    };
    return labels[kind] || kind.toUpperCase().slice(0, 4);
  }

  function getKindClass(kind: string): string {
    const classes: Record<string, string> = {
      message: "bg-blue text-surface",
      tool_call: "bg-purple text-surface",
      tool_result: "bg-green text-surface",
      error: "bg-red text-surface",
      system: "bg-surface-muted text-fg-dim",
    };
    return classes[kind] || "bg-surface-muted text-fg";
  }

  function truncateContent(content: string | null, maxLen: number = 120): string {
    if (!content) return "(no content)";
    const cleaned = content.replaceAll(String.raw`\n`, " ").replaceAll(/\\s+/g, " ");
    if (cleaned.length <= maxLen) return cleaned;
    return cleaned.slice(0, maxLen) + "...";
  }

  onMount(() => {
    loadFacets();
    loadActivityStats();
  });
</script>

<div class="flex h-full flex-col overflow-hidden">
  <div class="border-surface-muted bg-surface-soft border-b p-4">
    <div class="mb-3 flex gap-2">
      <input
        type="text"
        class="bg-surface border-surface-muted text-fg font-inherit focus:border-blue flex-1 rounded border px-3 py-2 text-sm focus:outline-none"
        placeholder="Search across sessions..."
        bind:value={query}
        onkeydown={handleKeydown} />
      <button
        class="bg-blue text-surface font-inherit hover:not-disabled:bg-blue-bright cursor-pointer rounded border-none px-4 py-2 text-sm transition-colors disabled:cursor-not-allowed disabled:opacity-50"
        onclick={performSearch}
        disabled={loading}>
        {loading ? "Searching..." : "Search"}
      </button>
    </div>

    <div class="flex gap-2">
      <button
        class="border-surface-muted text-fg-dim font-inherit hover:border-blue hover:text-fg cursor-pointer rounded border bg-transparent px-3 py-1.5 text-xs transition-all"
        onclick={() => (showFilters = !showFilters)}>
        {showFilters ? "Hide Filters" : "Show Filters"}
      </button>
      <button
        class="border-surface-muted text-fg-dim font-inherit hover:border-blue hover:text-fg cursor-pointer rounded border bg-transparent px-3 py-1.5 text-xs transition-all"
        onclick={() => (showAnalytics = !showAnalytics)}>
        {showAnalytics ? "Hide Analytics" : "Show Analytics"}
      </button>
    </div>
  </div>

  {#if showFilters}
    <div class="bg-surface border-surface-muted grid grid-cols-[repeat(auto-fit,minmax(150px,1fr))] gap-4 border-b p-4">
      <div class="flex flex-col gap-1">
        <label for="since-filter" class="text-fg-dim text-xs tracking-wide uppercase">Since</label>
        <select
          id="since-filter"
          class="bg-surface-soft border-surface-muted text-fg font-inherit focus:border-blue cursor-pointer rounded border px-2 py-1.5 text-sm focus:outline-none"
          value={facets.since || ""}
          onchange={(e) => {
            facets.since = e.currentTarget.value || undefined;
            performSearch();
          }}>
          <option value="">All sources</option>
          {#each availableSources as source (source)}
            <option value={source}>{source}</option>
          {/each}
        </select>
      </div>

      <div class="flex flex-col gap-1">
        <label for="project-filter" class="text-fg-dim text-xs tracking-wide uppercase">Project</label>
        <select
          id="project-filter"
          class="bg-surface-soft border-surface-muted text-fg font-inherit focus:border-blue cursor-pointer rounded border px-2 py-1.5 text-sm focus:outline-none"
          value={facets.project || ""}
          onchange={(e) => {
            facets.project = e.currentTarget.value || undefined;
            performSearch();
          }}>
          <option value="">All projects</option>
          {#each availableProjects as project (project)}
            <option value={project}>{getDisplayProject(project)}</option>
          {/each}
        </select>
      </div>

      <div class="flex flex-col gap-1">
        <label for="kind-filter" class="text-fg-dim text-xs tracking-wide uppercase">Event Kind</label>
        <select
          id="kind-filter"
          class="bg-surface-soft border-surface-muted text-fg font-inherit focus:border-blue cursor-pointer rounded border px-2 py-1.5 text-sm focus:outline-none"
          value={facets.kind || ""}
          onchange={(e) => {
            facets.kind = e.currentTarget.value || undefined;
            performSearch();
          }}>
          <option value="">All kinds</option>
          {#each availableKinds as kind (kind)}
            <option value={kind}>{kind}</option>
          {/each}
        </select>
      </div>

      <div class="flex flex-col gap-1">
        <label for="since-filter" class="text-fg-dim text-xs tracking-wide uppercase">Since</label>
        <select
          id="since-filter"
          class="bg-surface-soft border-surface-muted text-fg font-inherit focus:border-blue cursor-pointer rounded border px-2 py-1.5 text-sm focus:outline-none"
          value={facets.since || ""}
          onchange={(e) => {
            facets.since = e.currentTarget.value || undefined;
            performSearch();
          }}>
          <option value="">All time</option>
          <option value="1d">Last 24 hours</option>
          <option value="7d">Last 7 days</option>
          <option value="30d">Last 30 days</option>
          <option value="90d">Last 90 days</option>
        </select>
      </div>

      <button
        class="border-surface-muted text-fg-dim font-inherit hover:border-red hover:text-red cursor-pointer self-end rounded border bg-transparent px-3 py-1.5 text-xs transition-colors"
        onclick={clearFilters}>
        Clear Filters
      </button>
    </div>
  {/if}

  {#if showAnalytics && activityStats.length > 0}
    <div class="bg-surface-soft border-surface-muted border-b p-4">
      <h3 class="text-fg m-0 mb-3 text-sm font-semibold">Activity (Last 30 Days)</h3>
      <div class="relative flex h-25 items-end gap-1 pb-6">
        {#each activityStats.slice(0, 14) as stat (stat.day)}
          {@const maxEvents = Math.max(...activityStats.map((s) => s.event_count))}
          {@const barHeight = maxEvents > 0 ? (stat.event_count / maxEvents) * 100 : 0}
          <div class="group relative flex h-full flex-1 flex-col items-center">
            <div
              class="bg-blue group-hover:bg-blue-bright min-h-0.5 w-full rounded-t-sm transition-colors"
              style="height: {barHeight}%">
            </div>
            <div class="text-fg-dim absolute -bottom-5 origin-center -rotate-45 text-xs whitespace-nowrap">
              {stat.day.slice(5)}
            </div>
            <div
              class="bg-surface border-surface-muted text-fg pointer-events-none absolute bottom-full left-1/2 z-10 -translate-x-1/2 rounded border px-2 py-1 text-xs whitespace-nowrap opacity-0 transition-opacity group-hover:opacity-100">
              {stat.day}: {stat.event_count} events, {stat.session_count} sessions
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if error}
    <div class="bg-red text-surface mx-4 my-2 rounded p-2 text-xs">{error}</div>
  {/if}

  <div class="flex flex-1 flex-col overflow-hidden">
    {#if results.length > 0}
      <div class="border-surface-muted bg-surface-soft border-b px-4 py-2">
        <span class="text-fg-dim text-xs">{results.length} results</span>
      </div>

      <div class="flex-1 overflow-y-auto p-2">
        {#each results as result (result.event.id)}
          <div
            class="bg-surface-soft border-surface-muted hover:border-blue mb-2 cursor-pointer rounded border p-3 transition-colors"
            onclick={() => onSelectEvent?.(result.event)}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                onSelectEvent?.(result.event);
              }
            }}
            role="button"
            tabindex="0"
            aria-label="View event details">
            <div class="mb-2 flex items-center gap-2">
              <span class="text-fg-dim text-xs">{formatTimestamp(result.event.timestamp)}</span>
              <span class="rounded px-1.5 py-0.5 text-xs font-semibold uppercase {getKindClass(result.event.kind)}">
                {getKindLabel(result.event.kind)}
              </span>
              {#if result.event.role}
                <span class="text-fg-dim text-xs lowercase">{result.event.role}</span>
              {/if}
            </div>
            <div class="text-fg mb-2 text-sm leading-snug wrap-break-word">
              {truncateContent(result.event.content)}
            </div>
            <div class="flex items-center justify-between">
              <button
                class="font-inherit text-blue hover:text-blue-bright cursor-pointer border-none bg-transparent p-0 text-xs underline"
                onclick={() => onSelectSession?.(result.event.session_id)}>
                Session: {result.event.session_id.slice(0, 8)}
              </button>
              <span class="text-fg-dim text-xs">rank: {result.rank.toFixed(4)}</span>
            </div>
          </div>
        {/each}
      </div>
    {:else if query && !loading}
      <div class="text-fg-dim flex flex-1 flex-col items-center justify-center p-8 text-center">
        <p>No results found for "{query}"</p>
        {#if Object.keys(facets).length > 0}
          <p class="mt-2 text-sm">Try adjusting your filters</p>
        {/if}
      </div>
    {:else if !query}
      <div class="text-fg-dim flex flex-1 flex-col items-center justify-center p-8 text-center">
        <p>Enter a search query to find events across sessions</p>
        <p class="mt-2 text-sm">Search supports full-text matching on event content</p>
      </div>
    {/if}
  </div>
</div>
