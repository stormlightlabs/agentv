<script lang="ts">
  import type { ActivityStats, SearchFacets, SearchResult } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type Props = { onSelectSession?: (sessionId: string) => void };

  let { onSelectSession }: Props = $props();

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
    } catch (e) {
      console.error("Failed to load facets:", e);
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
    } catch (e) {
      error = String(e);
      results = [];
    } finally {
      loading = false;
    }
  }

  async function loadActivityStats() {
    try {
      const stats: ActivityStats[] = await invoke("get_activity_stats", { since: "30d", until: null });
      activityStats = stats;
    } catch (e) {
      console.error("Failed to load activity stats:", e);
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
      message: "bg-blue text-bg",
      tool_call: "bg-purple text-bg",
      tool_result: "bg-green text-bg",
      error: "bg-red text-bg",
      system: "bg-bg-muted text-fg-dim",
    };
    return classes[kind] || "bg-bg-muted text-fg";
  }

  function truncateContent(content: string | null, maxLen: number = 120): string {
    if (!content) return "(no content)";
    const cleaned = content.replace(/\\n/g, " ").replace(/\\s+/g, " ");
    if (cleaned.length <= maxLen) return cleaned;
    return cleaned.slice(0, maxLen) + "...";
  }

  onMount(() => {
    loadFacets();
    loadActivityStats();
  });
</script>

<div class="flex flex-col h-full overflow-hidden">
  <div class="p-4 border-b border-bg-muted bg-bg-soft">
    <div class="flex gap-2 mb-3">
      <input
        type="text"
        class="flex-1 px-3 py-2 bg-bg border border-bg-muted rounded text-fg font-inherit text-sm focus:outline-none focus:border-blue"
        placeholder="Search across sessions..."
        bind:value={query}
        onkeydown={handleKeydown} />
      <button
        class="px-4 py-2 bg-blue text-bg border-none rounded font-inherit text-sm cursor-pointer transition-colors hover:not-disabled:bg-blue-bright disabled:opacity-50 disabled:cursor-not-allowed"
        onclick={performSearch}
        disabled={loading}>
        {loading ? "Searching..." : "Search"}
      </button>
    </div>

    <div class="flex gap-2">
      <button
        class="px-3 py-1.5 bg-transparent border border-bg-muted rounded text-fg-dim font-inherit text-xs cursor-pointer transition-all hover:border-blue hover:text-fg"
        onclick={() => (showFilters = !showFilters)}>
        {showFilters ? "Hide Filters" : "Show Filters"}
      </button>
      <button
        class="px-3 py-1.5 bg-transparent border border-bg-muted rounded text-fg-dim font-inherit text-xs cursor-pointer transition-all hover:border-blue hover:text-fg"
        onclick={() => (showAnalytics = !showAnalytics)}>
        {showAnalytics ? "Hide Analytics" : "Show Analytics"}
      </button>
    </div>
  </div>

  {#if showFilters}
    <div class="grid grid-cols-[repeat(auto-fit,minmax(150px,1fr))] gap-4 p-4 bg-bg border-b border-bg-muted">
      <div class="flex flex-col gap-1">
        <label for="since-filter" class="text-xs text-fg-dim uppercase tracking-wide">Since</label>
        <select
          id="since-filter"
          class="px-2 py-1.5 bg-bg-soft border border-bg-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
          value={facets.since || ""}
          onchange={(e) => {
            facets.since = e.currentTarget.value || undefined;
            performSearch();
          }}>
          <option value="">All sources</option>
          {#each availableSources as source}
            <option value={source}>{source}</option>
          {/each}
        </select>
      </div>

      <div class="flex flex-col gap-1">
        <label for="project-filter" class="text-xs text-fg-dim uppercase tracking-wide">Project</label>
        <select
          id="project-filter"
          class="px-2 py-1.5 bg-bg-soft border border-bg-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
          value={facets.project || ""}
          onchange={(e) => {
            facets.project = e.currentTarget.value || undefined;
            performSearch();
          }}>
          <option value="">All projects</option>
          {#each availableProjects as project}
            <option value={project}>{project}</option>
          {/each}
        </select>
      </div>

      <div class="flex flex-col gap-1">
        <label for="kind-filter" class="text-xs text-fg-dim uppercase tracking-wide">Event Kind</label>
        <select
          id="kind-filter"
          class="px-2 py-1.5 bg-bg-soft border border-bg-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
          value={facets.kind || ""}
          onchange={(e) => {
            facets.kind = e.currentTarget.value || undefined;
            performSearch();
          }}>
          <option value="">All kinds</option>
          {#each availableKinds as kind}
            <option value={kind}>{kind}</option>
          {/each}
        </select>
      </div>

      <div class="flex flex-col gap-1">
        <label for="since-filter" class="text-xs text-fg-dim uppercase tracking-wide">Since</label>
        <select
          id="since-filter"
          class="px-2 py-1.5 bg-bg-soft border border-bg-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
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
        class="px-3 py-1.5 bg-transparent border border-bg-muted rounded text-fg-dim font-inherit text-xs cursor-pointer self-end transition-colors hover:border-red hover:text-red"
        onclick={clearFilters}>
        Clear Filters
      </button>
    </div>
  {/if}

  {#if showAnalytics && activityStats.length > 0}
    <div class="p-4 bg-bg-soft border-b border-bg-muted">
      <h3 class="m-0 mb-3 text-sm font-semibold text-fg">Activity (Last 30 Days)</h3>
      <div class="flex items-end gap-1 h-25 pb-6 relative">
        {#each activityStats.slice(0, 14) as stat}
          {@const maxEvents = Math.max(...activityStats.map((s) => s.event_count))}
          {@const barHeight = maxEvents > 0 ? (stat.event_count / maxEvents) * 100 : 0}
          <div class="flex-1 flex flex-col items-center relative h-full group">
            <div
              class="w-full bg-blue rounded-t-sm min-h-0.5 transition-colors group-hover:bg-blue-bright"
              style="height: {barHeight}%">
            </div>
            <div class="absolute -bottom-5 text-2xs text-fg-dim -rotate-45 origin-center whitespace-nowrap">
              {stat.day.slice(5)}
            </div>
            <div
              class="absolute bottom-full left-1/2 -translate-x-1/2 px-2 py-1 bg-bg border border-bg-muted rounded text-xs text-fg whitespace-nowrap opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 z-10">
              {stat.day}: {stat.event_count} events, {stat.session_count} sessions
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if error}
    <div class="mx-4 my-2 p-2 bg-red text-bg rounded text-xs">{error}</div>
  {/if}

  <div class="flex-1 overflow-hidden flex flex-col">
    {#if results.length > 0}
      <div class="px-4 py-2 border-b border-bg-muted bg-bg-soft">
        <span class="text-xs text-fg-dim">{results.length} results</span>
      </div>

      <div class="flex-1 overflow-y-auto p-2">
        {#each results as result (result.event.id)}
          <div class="p-3 mb-2 bg-bg-soft border border-bg-muted rounded transition-colors hover:border-blue">
            <div class="flex items-center gap-2 mb-2">
              <span class="text-xs text-fg-dim">{formatTimestamp(result.event.timestamp)}</span>
              <span class="text-2xs font-semibold px-1.5 py-0.5 rounded uppercase {getKindClass(result.event.kind)}">
                {getKindLabel(result.event.kind)}
              </span>
              {#if result.event.role}
                <span class="text-2xs text-fg-dim lowercase">{result.event.role}</span>
              {/if}
            </div>
            <div class="text-sm text-fg leading-snug wrap-break-word mb-2">
              {truncateContent(result.event.content)}
            </div>
            <div class="flex justify-between items-center">
              <button
                class="bg-transparent border-none p-0 font-inherit text-xs text-blue cursor-pointer underline hover:text-blue-bright"
                onclick={() => onSelectSession?.(result.event.session_id)}>
                Session: {result.event.session_id.slice(0, 8)}
              </button>
              <span class="text-2xs text-fg-dim">rank: {result.rank.toFixed(4)}</span>
            </div>
          </div>
        {/each}
      </div>
    {:else if query && !loading}
      <div class="flex-1 flex flex-col items-center justify-center text-fg-dim text-center p-8">
        <p>No results found for "{query}"</p>
        {#if Object.keys(facets).length > 0}
          <p class="text-sm mt-2">Try adjusting your filters</p>
        {/if}
      </div>
    {:else if !query}
      <div class="flex-1 flex flex-col items-center justify-center text-fg-dim text-center p-8">
        <p>Enter a search query to find events across sessions</p>
        <p class="text-sm mt-2">Search supports full-text matching on event content</p>
      </div>
    {/if}
  </div>
</div>
