<script lang="ts">
  import { filterStore, type FilterState } from "$lib/stores/filters.svelte";
  import { untrack } from "svelte";
  import { fly } from "svelte/transition";

  type Props = {
    sources: string[];
    projects: string[];
    kinds: string[];
    roles?: string[];
    tools?: string[];
    showSearch?: boolean;
    showDateRange?: boolean;
    onSearch?: (filters: FilterState) => void;
    onChange?: (filters: FilterState) => void;
  };

  let {
    sources = [],
    projects = [],
    kinds = [],
    roles = [],
    tools = [],
    showSearch = true,
    showDateRange = true,
    onSearch,
    onChange,
  }: Props = $props();

  let showFilters = $state(false);
  let searchInput: HTMLInputElement | null = $state(null);
  const activeCount = $derived(filterStore.activeCount);

  const roleOptions = ["user", "assistant", "system"];
  const dateOptions = [
    { value: "", label: "All time" },
    { value: "1h", label: "Last hour" },
    { value: "1d", label: "Last 24 hours" },
    { value: "7d", label: "Last 7 days" },
    { value: "30d", label: "Last 30 days" },
    { value: "90d", label: "Last 90 days" },
  ];

  function handleSearch() {
    onSearch?.(filterStore.state);
  }

  function handleFilterChange(key: keyof FilterState, value: string | null) {
    filterStore.setFilter(key, value);
    onChange?.(filterStore.state);
  }

  function clearAllFilters() {
    filterStore.clearAll();
    onChange?.(filterStore.state);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && showSearch) {
      handleSearch();
    }
    if (event.key === "Escape") {
      showFilters = false;
    }
    if ((event.metaKey || event.ctrlKey) && event.key === "k") {
      event.preventDefault();
      searchInput?.focus();
    }
  }

  function getActiveFilterLabel(key: keyof FilterState, value: string | null): string {
    if (!value) return "";
    switch (key) {
      case "since":
        return dateOptions.find((o) => o.value === value)?.label || value;
      default:
        return value;
    }
  }

  $effect(() => {
    untrack(() => {
      if (activeCount > 0 && !showFilters) {
        showFilters = true;
      }
    });
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex flex-col gap-3 p-4 bg-surface-soft border-b border-surface-muted">
  <div class="flex gap-2">
    {#if showSearch}
      <div class="flex-1 relative">
        <input
          bind:this={searchInput}
          type="text"
          class="w-full px-3 py-2 pl-9 bg-surface border border-surface-muted rounded text-fg font-inherit text-sm focus:outline-none focus:border-blue"
          placeholder="Search across events... (Cmd+K)"
          bind:value={filterStore.state.query}
          onkeydown={handleKeydown} />
        <span class="absolute left-3 top-1/2 -translate-y-1/2 i-ri-search-line text-fg-muted"></span>
        {#if filterStore.state.query}
          <button
            class="absolute right-3 top-1/2 -translate-y-1/2 text-fg-muted hover:text-fg cursor-pointer"
            onclick={() => handleFilterChange("query", "")}
            aria-label="Clear search"
            type="button">
            <span class="i-ri-close-line"></span>
          </button>
        {/if}
      </div>
      <button
        class="px-4 py-2 bg-blue text-surface border-none rounded font-inherit text-sm cursor-pointer transition-colors hover:not-disabled:bg-blue-bright disabled:opacity-50"
        onclick={handleSearch}>
        Search
      </button>
    {/if}

    <button
      class="px-3 py-2 bg-transparent border border-surface-muted rounded text-fg-dim font-inherit text-sm cursor-pointer transition-all hover:border-blue hover:text-fg flex items-center gap-2"
      onclick={() => (showFilters = !showFilters)}
      disabled={activeCount > 0}>
      <span class="i-ri-filter-3-line"></span>
      Filters
      {#if filterStore.activeCount > 0}
        <span class="bg-blue text-surface text-xs px-1.5 py-0.5 rounded">{filterStore.activeCount}</span>
      {/if}
    </button>
  </div>

  {#if filterStore.activeCount > 0}
    <div class="flex flex-wrap gap-2 items-center">
      <span class="text-xs text-fg-muted">Active filters:</span>
      {#if filterStore.state.source}
        <button
          class="inline-flex items-center gap-1 px-2 py-1 bg-blue/20 text-blue rounded text-xs cursor-pointer hover:bg-blue/30 transition-colors"
          onclick={() => handleFilterChange("source", null)}>
          Source: {filterStore.state.source}
          <span class="i-ri-close-line"></span>
        </button>
      {/if}
      {#if filterStore.state.project}
        <button
          class="inline-flex items-center gap-1 px-2 py-1 bg-purple/20 text-purple rounded text-xs cursor-pointer hover:bg-purple/30 transition-colors"
          onclick={() => handleFilterChange("project", null)}>
          Project: {filterStore.state.project}
          <span class="i-ri-close-line"></span>
        </button>
      {/if}
      {#if filterStore.state.kind}
        <button
          class="inline-flex items-center gap-1 px-2 py-1 bg-green/20 text-green rounded text-xs cursor-pointer hover:bg-green/30 transition-colors"
          onclick={() => handleFilterChange("kind", null)}>
          Kind: {filterStore.state.kind}
          <span class="i-ri-close-line"></span>
        </button>
      {/if}
      {#if filterStore.state.role}
        <button
          class="inline-flex items-center gap-1 px-2 py-1 bg-yellow/20 text-yellow rounded text-xs cursor-pointer hover:bg-yellow/30 transition-colors"
          onclick={() => handleFilterChange("role", null)}>
          Role: {filterStore.state.role}
          <span class="i-ri-close-line"></span>
        </button>
      {/if}
      {#if filterStore.state.tool}
        <button
          class="inline-flex items-center gap-1 px-2 py-1 bg-red/20 text-red rounded text-xs cursor-pointer hover:bg-red/30 transition-colors"
          onclick={() => handleFilterChange("tool", null)}>
          Tool: {filterStore.state.tool}
          <span class="i-ri-close-line"></span>
        </button>
      {/if}
      {#if filterStore.state.since}
        <button
          class="inline-flex items-center gap-1 px-2 py-1 bg-cyan/20 text-cyan rounded text-xs cursor-pointer hover:bg-cyan/30 transition-colors"
          onclick={() => handleFilterChange("since", null)}>
          Since: {getActiveFilterLabel("since", filterStore.state.since)}
          <span class="i-ri-close-line"></span>
        </button>
      {/if}
      <button class="text-xs text-fg-muted hover:text-red cursor-pointer transition-colors" onclick={clearAllFilters}>
        Clear all
      </button>
    </div>
  {/if}

  {#if showFilters}
    <div
      class="grid grid-cols-[repeat(auto-fit,minmax(180px,1fr))] gap-4 pt-3 border-t border-surface-muted"
      transition:fly={{ y: -10, duration: 200 }}>
      <div class="flex flex-col gap-1">
        <label for="filter-source" class="text-xs text-fg-dim uppercase tracking-wide">Source</label>
        <select
          id="filter-source"
          class="px-2 py-1.5 bg-surface border border-surface-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
          value={filterStore.state.source || ""}
          onchange={(e) => handleFilterChange("source", e.currentTarget.value || null)}>
          <option value="">All sources</option>
          {#each sources as source}
            <option value={source}>{source}</option>
          {/each}
        </select>
      </div>

      <div class="flex flex-col gap-1">
        <label for="filter-project" class="text-xs text-fg-dim uppercase tracking-wide">Project</label>
        <select
          id="filter-project"
          class="px-2 py-1.5 bg-surface border border-surface-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
          value={filterStore.state.project || ""}
          onchange={(e) => handleFilterChange("project", e.currentTarget.value || null)}>
          <option value="">All projects</option>
          {#each projects as project}
            <option value={project}>{project}</option>
          {/each}
        </select>
      </div>

      <div class="flex flex-col gap-1">
        <label for="filter-kind" class="text-xs text-fg-dim uppercase tracking-wide">Event Kind</label>
        <select
          id="filter-kind"
          class="px-2 py-1.5 bg-surface border border-surface-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
          value={filterStore.state.kind || ""}
          onchange={(e) => handleFilterChange("kind", e.currentTarget.value || null)}>
          <option value="">All kinds</option>
          {#each kinds as kind}
            <option value={kind}>{kind}</option>
          {/each}
        </select>
      </div>

      {#if roles.length > 0 || roleOptions.length > 0}
        <div class="flex flex-col gap-1">
          <label for="filter-role" class="text-xs text-fg-dim uppercase tracking-wide">Role</label>
          <select
            id="filter-role"
            class="px-2 py-1.5 bg-surface border border-surface-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
            value={filterStore.state.role || ""}
            onchange={(e) => handleFilterChange("role", e.currentTarget.value || null)}>
            <option value="">All roles</option>
            {#each roles.length > 0 ? roles : roleOptions as role}
              <option value={role}>{role}</option>
            {/each}
          </select>
        </div>
      {/if}

      {#if tools.length > 0}
        <div class="flex flex-col gap-1">
          <label for="filter-tool" class="text-xs text-fg-dim uppercase tracking-wide">Tool</label>
          <select
            id="filter-tool"
            class="px-2 py-1.5 bg-surface border border-surface-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
            value={filterStore.state.tool || ""}
            onchange={(e) => handleFilterChange("tool", e.currentTarget.value || null)}>
            <option value="">All tools</option>
            {#each tools as tool}
              <option value={tool}>{tool}</option>
            {/each}
          </select>
        </div>
      {/if}

      {#if showDateRange}
        <div class="flex flex-col gap-1">
          <label for="filter-since" class="text-xs text-fg-dim uppercase tracking-wide">Since</label>
          <select
            id="filter-since"
            class="px-2 py-1.5 bg-surface border border-surface-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
            value={filterStore.state.since || ""}
            onchange={(e) => handleFilterChange("since", e.currentTarget.value || null)}>
            {#each dateOptions as option}
              <option value={option.value}>{option.label}</option>
            {/each}
          </select>
        </div>
      {/if}
    </div>
  {/if}
</div>
