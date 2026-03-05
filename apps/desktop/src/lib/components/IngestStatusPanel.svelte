<script lang="ts">
  import type { SourceHealth } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";

  type Props = { onRefresh?: () => void };

  let { onRefresh }: Props = $props();

  let healthData = $state<SourceHealth[]>([]);
  let ingestHistory = $state<Record<string, IngestHistoryEntry>>({});
  let loading = $state(false);
  let lastChecked = $state<Date | null>(null);
  let showStaleAlert = $state(false);

  type IngestHistoryEntry = {
    lastSuccess: Date;
    lastFailure: Date | null;
    failureCount: number;
    successCount: number;
    lastDurationMs: number;
    lastImported: number;
    lastFailed: number;
    status: "fresh" | "stale" | "failed";
  };

  const STALE_THRESHOLD_MS = 5 * 60 * 1000;

  function syncIngestHistoryFromHealth(sources: SourceHealth[]) {
    const now = new Date();
    const nextHistory: Record<string, IngestHistoryEntry> = { ...ingestHistory };

    for (const source of sources) {
      const previous = nextHistory[source.source];
      const isFailed = source.status === "unhealthy";
      const isStale = source.status === "degraded";

      nextHistory[source.source] = {
        lastSuccess: isFailed ? (previous?.lastSuccess ?? now) : now,
        lastFailure: isFailed ? now : (previous?.lastFailure ?? null),
        failureCount: (previous?.failureCount ?? 0) + (isFailed && previous?.status !== "failed" ? 1 : 0),
        successCount: (previous?.successCount ?? 0) + (!isFailed && previous?.status === "failed" ? 1 : 0),
        lastDurationMs: previous?.lastDurationMs ?? 0,
        lastImported: previous?.lastImported ?? 0,
        lastFailed: isFailed ? 1 : 0,
        status: isFailed ? "failed" : isStale ? "stale" : "fresh",
      };
    }

    ingestHistory = nextHistory;
  }

  async function loadHealth() {
    loading = true;
    try {
      healthData = await invoke<SourceHealth[]>("get_source_health");
      syncIngestHistoryFromHealth(healthData);
      lastChecked = new Date();
      onRefresh?.();
      checkStaleSources();
    } catch (error) {
      console.error("Failed to load source health:", error);
    } finally {
      loading = false;
    }
  }

  function checkStaleSources() {
    const now = Date.now();
    const nextHistory: Record<string, IngestHistoryEntry> = { ...ingestHistory };
    let hasStale = false;

    for (const source of Object.keys(nextHistory)) {
      const entry = nextHistory[source];
      const lastSuccessMs = entry.lastSuccess.getTime();
      const isStale = now - lastSuccessMs > STALE_THRESHOLD_MS;

      if (isStale && entry.status !== "failed") {
        nextHistory[source] = { ...entry, status: "stale" };
        hasStale = true;
      } else if (!isStale && entry.status === "stale") {
        nextHistory[source] = { ...entry, status: "fresh" };
      } else if (entry.status === "stale") {
        hasStale = true;
      }
    }

    ingestHistory = nextHistory;
    showStaleAlert = hasStale;
  }

  function getStatusColor(status: SourceHealth["status"]): string {
    switch (status) {
      case "healthy": {
        return "bg-green";
      }
      case "degraded": {
        return "bg-yellow";
      }
      case "unhealthy": {
        return "bg-red";
      }
      default: {
        return "bg-fg-dim";
      }
    }
  }

  function getStatusIcon(status: SourceHealth["status"]): string {
    switch (status) {
      case "healthy": {
        return "i-ri-checkbox-circle-line";
      }
      case "degraded": {
        return "i-ri-alert-line";
      }
      case "unhealthy": {
        return "i-ri-close-circle-line";
      }
      default: {
        return "i-ri-question-line";
      }
    }
  }

  function getIngestStatusColor(status: IngestHistoryEntry["status"]): string {
    switch (status) {
      case "fresh": {
        return "text-green";
      }
      case "stale": {
        return "text-yellow";
      }
      case "failed": {
        return "text-red";
      }
    }
  }

  function getIngestStatusIcon(status: IngestHistoryEntry["status"]): string {
    switch (status) {
      case "fresh": {
        return "i-ri-check-double-line";
      }
      case "stale": {
        return "i-ri-time-line";
      }
      case "failed": {
        return "i-ri-error-warning-line";
      }
    }
  }

  function formatLastChecked(): string {
    if (!lastChecked) return "Never";
    const now = new Date();
    const diff = now.getTime() - lastChecked.getTime();
    const seconds = Math.floor(diff / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    return `${hours}h ago`;
  }

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  }

  function formatTimeAgo(date: Date): string {
    const diff = Date.now() - date.getTime();
    const seconds = Math.floor(diff / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }

  onMount(() => {
    loadHealth();
    const interval = setInterval(checkStaleSources, 60_000);
    return () => clearInterval(interval);
  });
</script>

<div class="flex h-full flex-col overflow-hidden p-4">
  {#if showStaleAlert}
    <div
      class="bg-yellow/20 border-yellow mb-4 flex items-center gap-3 rounded border px-4 py-3"
      transition:fly={{ y: -20 }}>
      <span class="i-ri-time-line text-yellow text-xl"></span>
      <div class="flex-1">
        <div class="text-yellow text-sm font-medium">Stale Sources Detected</div>
        <div class="text-fg-dim text-xs">Some sources haven't been ingested recently</div>
      </div>
    </div>
  {/if}

  <div class="mb-4 flex items-center justify-between">
    <h2 class="text-fg m-0 text-lg font-semibold">Ingest Status</h2>
    <button
      class="border-surface-muted text-fg-dim font-inherit hover:border-blue hover:text-fg flex cursor-pointer items-center gap-1 rounded border bg-transparent px-3 py-1.5 text-xs transition-all"
      onclick={loadHealth}
      disabled={loading}>
      <span class={loading ? "i-ri-loader-4-line animate-spin" : "i-ri-refresh-line"}></span>
      Refresh
    </button>
  </div>

  <div class="text-fg-dim mb-4 text-xs">
    Last checked: {formatLastChecked()}
  </div>

  <div class="flex-1 space-y-4 overflow-y-auto">
    {#each healthData as source, index (source.source)}
      {@const history = ingestHistory[source.source]}
      <div
        class="bg-surface-soft border-surface-muted hover:border-blue rounded-lg border p-4 transition-all"
        transition:fly={{ y: 10, duration: 200, delay: index * 50 }}>
        <div class="mb-3 flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="text-fg text-sm font-medium capitalize">{source.source}</span>
            <span class="text-surface rounded px-1.5 py-0.5 text-xs uppercase {getStatusColor(source.status)}">
              {source.status}
            </span>
          </div>
          <span class="{getStatusIcon(source.status)} {getStatusColor(source.status).replace('bg-', 'text-')}"></span>
        </div>

        {#if source.path}
          <div class="text-fg-dim mb-3 flex items-start gap-1 text-xs">
            <span class="i-ri-folder-line mt-0.5 shrink-0"></span>
            <span class="font-mono break-all">{source.path}</span>
          </div>
        {/if}

        {#if source.message}
          <div class="text-fg-dim mb-3 flex items-start gap-1 text-xs">
            <span class="i-ri-information-line mt-0.5 shrink-0"></span>
            <span class="break-all">{source.message}</span>
          </div>
        {/if}

        {#if history}
          <div class="border-surface-muted mt-3 border-t pt-3">
            <div class="mb-3 grid grid-cols-2 gap-3">
              <div class="flex items-center gap-2">
                <span class="{getIngestStatusIcon(history.status)} {getIngestStatusColor(history.status)}"></span>
                <span class="text-fg-dim text-xs">
                  {history.status === "fresh"
                    ? `Last: ${formatTimeAgo(history.lastSuccess)}`
                    : history.status === "stale"
                      ? `Stale for ${formatTimeAgo(new Date(Date.now() - 5 * 60 * 1000))}`
                      : "Last ingest failed"}
                </span>
              </div>
              <div class="text-fg-dim text-right text-xs">
                Duration: {formatDuration(history.lastDurationMs)}
              </div>
            </div>

            <div class="grid grid-cols-3 gap-2 text-center">
              <div class="bg-surface rounded p-2">
                <div class="text-green text-sm font-semibold">{history.successCount}</div>
                <div class="text-fg-dim text-xs">Successes</div>
              </div>
              <div class="bg-surface rounded p-2">
                <div class="text-red text-sm font-semibold">{history.failureCount}</div>
                <div class="text-fg-dim text-xs">Failures</div>
              </div>
              <div class="bg-surface rounded p-2">
                <div class="text-blue text-sm font-semibold">{history.lastImported}</div>
                <div class="text-fg-dim text-xs">Last Import</div>
              </div>
            </div>
          </div>
        {:else}
          <div class="border-surface-muted mt-3 border-t pt-3">
            <div class="text-fg-dim text-xs italic">No ingest history available</div>
          </div>
        {/if}
      </div>
    {:else}
      {#if !loading}
        <div class="text-center text-fg-dim py-8">
          <div class="i-ri-error-warning-line text-3xl mb-2 mx-auto"></div>
          <p class="text-sm">No health data available</p>
        </div>
      {/if}
    {/each}
  </div>

  <div class="bg-surface-soft border-surface-muted mt-4 rounded border p-3 text-xs">
    <div class="flex flex-wrap gap-4">
      <div class="flex items-center gap-1">
        <span class="bg-green h-2 w-2 rounded-full"></span>
        <span class="text-fg-dim">Healthy source</span>
      </div>
      <div class="flex items-center gap-1">
        <span class="bg-yellow h-2 w-2 rounded-full"></span>
        <span class="text-fg-dim">Stale (>5min)</span>
      </div>
      <div class="flex items-center gap-1">
        <span class="bg-red h-2 w-2 rounded-full"></span>
        <span class="text-fg-dim">Failed</span>
      </div>
    </div>
  </div>
</div>
