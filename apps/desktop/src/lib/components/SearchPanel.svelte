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
      message: "kind-message",
      tool_call: "kind-tool",
      tool_result: "kind-result",
      error: "kind-error",
      system: "kind-system",
    };
    return classes[kind] || "kind-default";
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

<div class="search-container">
  <div class="search-header">
    <div class="search-input-wrapper">
      <input
        type="text"
        class="search-input"
        placeholder="Search across sessions..."
        bind:value={query}
        onkeydown={handleKeydown} />
      <button class="search-button" onclick={performSearch} disabled={loading}>
        {loading ? "Searching..." : "Search"}
      </button>
    </div>

    <div class="search-actions">
      <button class="action-button" onclick={() => (showFilters = !showFilters)}>
        {showFilters ? "Hide Filters" : "Show Filters"}
      </button>
      <button class="action-button" onclick={() => (showAnalytics = !showAnalytics)}>
        {showAnalytics ? "Hide Analytics" : "Show Analytics"}
      </button>
    </div>
  </div>

  {#if showFilters}
    <div class="filters-panel">
      <div class="filter-group">
        <label class="filter-label">Source</label>
        <select
          class="filter-select"
          value={facets.source || ""}
          onchange={(e) => {
            facets.source = e.currentTarget.value || undefined;
            performSearch();
          }}>
          <option value="">All sources</option>
          {#each availableSources as source}
            <option value={source}>{source}</option>
          {/each}
        </select>
      </div>

      <div class="filter-group">
        <label class="filter-label">Project</label>
        <select
          class="filter-select"
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

      <div class="filter-group">
        <label class="filter-label">Event Kind</label>
        <select
          class="filter-select"
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

      <div class="filter-group">
        <label class="filter-label">Since</label>
        <select
          class="filter-select"
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

      <button class="clear-filters" onclick={clearFilters}>Clear Filters</button>
    </div>
  {/if}

  {#if showAnalytics && activityStats.length > 0}
    <div class="analytics-panel">
      <h3 class="analytics-title">Activity (Last 30 Days)</h3>
      <div class="activity-chart">
        {#each activityStats.slice(0, 14) as stat}
          {@const maxEvents = Math.max(...activityStats.map((s) => s.event_count))}
          {@const barHeight = maxEvents > 0 ? (stat.event_count / maxEvents) * 100 : 0}
          <div class="activity-bar-wrapper">
            <div class="activity-bar" style="height: {barHeight}%"></div>
            <div class="activity-label">{stat.day.slice(5)}</div>
            <div class="activity-tooltip">
              {stat.day}: {stat.event_count} events, {stat.session_count} sessions
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if error}
    <div class="error-message">{error}</div>
  {/if}

  <div class="results-container">
    {#if results.length > 0}
      <div class="results-header">
        <span class="results-count">{results.length} results</span>
      </div>

      <div class="results-list">
        {#each results as result (result.event.id)}
          <div class="result-item">
            <div class="result-header">
              <span class="result-timestamp">{formatTimestamp(result.event.timestamp)}</span>
              <span class="result-kind {getKindClass(result.event.kind)}">
                {getKindLabel(result.event.kind)}
              </span>
              {#if result.event.role}
                <span class="result-role">{result.event.role}</span>
              {/if}
            </div>
            <div class="result-content">
              {truncateContent(result.event.content)}
            </div>
            <div class="result-footer">
              <button class="session-link" onclick={() => onSelectSession?.(result.event.session_id)}>
                Session: {result.event.session_id.slice(0, 8)}
              </button>
              <span class="result-rank">rank: {result.rank.toFixed(4)}</span>
            </div>
          </div>
        {/each}
      </div>
    {:else if query && !loading}
      <div class="empty-state">
        <p>No results found for "{query}"</p>
        {#if Object.keys(facets).length > 0}
          <p class="empty-hint">Try adjusting your filters</p>
        {/if}
      </div>
    {:else if !query}
      <div class="empty-state">
        <p>Enter a search query to find events across sessions</p>
        <p class="empty-hint">Search supports full-text matching on event content</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .search-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .search-header {
    padding: 1rem;
    border-bottom: 1px solid var(--color-bg-muted);
    background-color: var(--color-bg-soft);
  }

  .search-input-wrapper {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }

  .search-input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    background-color: var(--color-bg);
    border: 1px solid var(--color-bg-muted);
    border-radius: 4px;
    color: var(--color-fg);
    font-family: inherit;
    font-size: 0.875rem;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--color-blue);
  }

  .search-button {
    padding: 0.5rem 1rem;
    background-color: var(--color-blue);
    color: var(--color-bg);
    border: none;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.875rem;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .search-button:hover:not(:disabled) {
    background-color: var(--color-blue-bright);
  }

  .search-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .search-actions {
    display: flex;
    gap: 0.5rem;
  }

  .action-button {
    padding: 0.375rem 0.75rem;
    background-color: transparent;
    border: 1px solid var(--color-bg-muted);
    border-radius: 4px;
    color: var(--color-fg-dim);
    font-family: inherit;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-button:hover {
    border-color: var(--color-blue);
    color: var(--color-fg);
  }

  .filters-panel {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 1rem;
    padding: 1rem;
    background-color: var(--color-bg);
    border-bottom: 1px solid var(--color-bg-muted);
  }

  .filter-group {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .filter-label {
    font-size: 0.75rem;
    color: var(--color-fg-dim);
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .filter-select {
    padding: 0.375rem 0.5rem;
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-bg-muted);
    border-radius: 4px;
    color: var(--color-fg);
    font-family: inherit;
    font-size: 0.875rem;
    cursor: pointer;
  }

  .filter-select:focus {
    outline: none;
    border-color: var(--color-blue);
  }

  .clear-filters {
    padding: 0.375rem 0.75rem;
    background-color: transparent;
    border: 1px solid var(--color-bg-muted);
    border-radius: 4px;
    color: var(--color-fg-dim);
    font-family: inherit;
    font-size: 0.75rem;
    cursor: pointer;
    align-self: flex-end;
  }

  .clear-filters:hover {
    border-color: var(--color-red);
    color: var(--color-red);
  }

  .analytics-panel {
    padding: 1rem;
    background-color: var(--color-bg-soft);
    border-bottom: 1px solid var(--color-bg-muted);
  }

  .analytics-title {
    margin: 0 0 0.75rem 0;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-fg);
  }

  .activity-chart {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    height: 100px;
    padding-bottom: 1.5rem;
    position: relative;
  }

  .activity-bar-wrapper {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    position: relative;
    height: 100%;
  }

  .activity-bar {
    width: 100%;
    background-color: var(--color-blue);
    border-radius: 2px 2px 0 0;
    min-height: 2px;
    transition: background-color 0.2s;
  }

  .activity-bar-wrapper:hover .activity-bar {
    background-color: var(--color-blue-bright);
  }

  .activity-label {
    position: absolute;
    bottom: -1.25rem;
    font-size: 0.625rem;
    color: var(--color-fg-dim);
    transform: rotate(-45deg);
    transform-origin: center;
    white-space: nowrap;
  }

  .activity-tooltip {
    position: absolute;
    bottom: 100%;
    left: 50%;
    transform: translateX(-50%);
    padding: 0.25rem 0.5rem;
    background-color: var(--color-bg);
    border: 1px solid var(--color-bg-muted);
    border-radius: 4px;
    font-size: 0.75rem;
    color: var(--color-fg);
    white-space: nowrap;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.2s;
    z-index: 10;
  }

  .activity-bar-wrapper:hover .activity-tooltip {
    opacity: 1;
  }

  .error-message {
    margin: 0.5rem 1rem;
    padding: 0.5rem;
    background-color: var(--color-red);
    color: var(--color-bg);
    border-radius: 4px;
    font-size: 0.75rem;
  }

  .results-container {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .results-header {
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--color-bg-muted);
    background-color: var(--color-bg-soft);
  }

  .results-count {
    font-size: 0.75rem;
    color: var(--color-fg-dim);
  }

  .results-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .result-item {
    padding: 0.75rem;
    margin-bottom: 0.5rem;
    background-color: var(--color-bg-soft);
    border: 1px solid var(--color-bg-muted);
    border-radius: 4px;
    transition: border-color 0.2s;
  }

  .result-item:hover {
    border-color: var(--color-blue);
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }

  .result-timestamp {
    font-size: 0.75rem;
    color: var(--color-fg-dim);
  }

  .result-kind {
    font-size: 0.625rem;
    font-weight: 600;
    padding: 0.125rem 0.375rem;
    border-radius: 3px;
    text-transform: uppercase;
  }

  .kind-message {
    background-color: var(--color-blue);
    color: var(--color-bg);
  }

  .kind-tool {
    background-color: var(--color-purple);
    color: var(--color-bg);
  }

  .kind-result {
    background-color: var(--color-green);
    color: var(--color-bg);
  }

  .kind-error {
    background-color: var(--color-red);
    color: var(--color-bg);
  }

  .kind-system {
    background-color: var(--color-bg-muted);
    color: var(--color-fg-dim);
  }

  .kind-default {
    background-color: var(--color-bg-muted);
    color: var(--color-fg);
  }

  .result-role {
    font-size: 0.625rem;
    color: var(--color-fg-dim);
    text-transform: lowercase;
  }

  .result-content {
    font-size: 0.875rem;
    color: var(--color-fg);
    line-height: 1.4;
    word-break: break-word;
    margin-bottom: 0.5rem;
  }

  .result-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .session-link {
    background: none;
    border: none;
    padding: 0;
    font-family: inherit;
    font-size: 0.75rem;
    color: var(--color-blue);
    cursor: pointer;
    text-decoration: underline;
  }

  .session-link:hover {
    color: var(--color-blue-bright);
  }

  .result-rank {
    font-size: 0.625rem;
    color: var(--color-fg-dim);
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--color-fg-dim);
    text-align: center;
    padding: 2rem;
  }

  .empty-hint {
    font-size: 0.875rem;
    margin-top: 0.5rem;
  }
</style>
