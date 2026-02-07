<script lang="ts">
  import type { IngestResult, SourceHealth } from "$lib/types";
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

  async function loadHealth() {
    loading = true;
    try {
      healthData = await invoke<SourceHealth[]>("get_source_health");
      lastChecked = new Date();
      onRefresh?.();
      checkStaleSources();
    } catch (e) {
      console.error("Failed to load source health:", e);
    } finally {
      loading = false;
    }
  }

  function recordIngestResult(source: string, result: IngestResult) {
    const now = new Date();
    const entry: IngestHistoryEntry = {
      lastSuccess: result.imported > 0 ? now : (ingestHistory[source]?.lastSuccess ?? now),
      lastFailure: result.failed > 0 ? now : null,
      failureCount: (ingestHistory[source]?.failureCount || 0) + (result.failed > 0 ? 1 : 0),
      successCount: (ingestHistory[source]?.successCount || 0) + (result.imported > 0 ? 1 : 0),
      lastDurationMs: result.duration_ms,
      lastImported: result.imported,
      lastFailed: result.failed,
      status: result.failed > 0 ? "failed" : "fresh",
    };

    ingestHistory = { ...ingestHistory, [source]: entry };
    checkStaleSources();
  }

  function checkStaleSources() {
    const now = new Date().getTime();
    let hasStale = false;

    for (const source of Object.keys(ingestHistory)) {
      const entry = ingestHistory[source];
      const lastSuccessMs = entry.lastSuccess.getTime();
      const isStale = now - lastSuccessMs > STALE_THRESHOLD_MS;

      if (isStale && entry.status !== "failed") {
        entry.status = "stale";
        hasStale = true;
      }
    }

    showStaleAlert = hasStale;
  }

  function getStatusColor(status: SourceHealth["status"]): string {
    switch (status) {
      case "healthy":
        return "bg-green";
      case "degraded":
        return "bg-yellow";
      case "unhealthy":
        return "bg-red";
      default:
        return "bg-fg-dim";
    }
  }

  function getStatusIcon(status: SourceHealth["status"]): string {
    switch (status) {
      case "healthy":
        return "i-ri-checkbox-circle-line";
      case "degraded":
        return "i-ri-alert-line";
      case "unhealthy":
        return "i-ri-close-circle-line";
      default:
        return "i-ri-question-line";
    }
  }

  function getIngestStatusColor(status: IngestHistoryEntry["status"]): string {
    switch (status) {
      case "fresh":
        return "text-green";
      case "stale":
        return "text-yellow";
      case "failed":
        return "text-red";
    }
  }

  function getIngestStatusIcon(status: IngestHistoryEntry["status"]): string {
    switch (status) {
      case "fresh":
        return "i-ri-check-double-line";
      case "stale":
        return "i-ri-time-line";
      case "failed":
        return "i-ri-error-warning-line";
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
    const diff = new Date().getTime() - date.getTime();
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
    const interval = setInterval(checkStaleSources, 60000);
    return () => clearInterval(interval);
  });
</script>

<div class="flex flex-col h-full overflow-hidden p-4">
  {#if showStaleAlert}
    <div
      class="mb-4 px-4 py-3 bg-yellow/20 border border-yellow rounded flex items-center gap-3"
      transition:fly={{ y: -20 }}>
      <span class="i-ri-time-line text-yellow text-xl"></span>
      <div class="flex-1">
        <div class="text-sm font-medium text-yellow">Stale Sources Detected</div>
        <div class="text-xs text-fg-dim">Some sources haven't been ingested recently</div>
      </div>
    </div>
  {/if}

  <div class="flex items-center justify-between mb-4">
    <h2 class="text-lg font-semibold text-fg m-0">Ingest Status</h2>
    <button
      class="px-3 py-1.5 bg-transparent border border-bg-muted rounded text-fg-dim font-inherit text-xs cursor-pointer transition-all hover:border-blue hover:text-fg flex items-center gap-1"
      onclick={loadHealth}
      disabled={loading}>
      <span class={loading ? "i-ri-loader-4-line animate-spin" : "i-ri-refresh-line"}></span>
      Refresh
    </button>
  </div>

  <div class="text-xs text-fg-dim mb-4">
    Last checked: {formatLastChecked()}
  </div>

  <div class="space-y-4 flex-1 overflow-y-auto">
    {#each healthData as source, index (source.source)}
      {@const history = ingestHistory[source.source]}
      <div
        class="p-4 bg-bg-soft border border-bg-muted rounded-lg transition-all hover:border-blue"
        transition:fly={{ y: 10, duration: 200, delay: index * 50 }}>
        <div class="flex items-center justify-between mb-3">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium text-fg capitalize">{source.source}</span>
            <span class="text-2xs uppercase px-1.5 py-0.5 rounded text-bg {getStatusColor(source.status)}">
              {source.status}
            </span>
          </div>
          <span class="{getStatusIcon(source.status)} {getStatusColor(source.status).replace('bg-', 'text-')}"></span>
        </div>

        {#if source.path}
          <div class="text-xs text-fg-dim mb-3 flex items-start gap-1">
            <span class="i-ri-folder-line shrink-0 mt-0.5"></span>
            <span class="break-all font-mono">{source.path}</span>
          </div>
        {/if}

        {#if source.message}
          <div class="text-xs text-fg-dim mb-3 flex items-start gap-1">
            <span class="i-ri-information-line shrink-0 mt-0.5"></span>
            <span class="break-all">{source.message}</span>
          </div>
        {/if}

        {#if history}
          <div class="border-t border-bg-muted pt-3 mt-3">
            <div class="grid grid-cols-2 gap-3 mb-3">
              <div class="flex items-center gap-2">
                <span class="{getIngestStatusIcon(history.status)} {getIngestStatusColor(history.status)}"></span>
                <span class="text-xs text-fg-dim">
                  {history.status === "fresh"
                    ? `Last: ${formatTimeAgo(history.lastSuccess)}`
                    : history.status === "stale"
                      ? `Stale for ${formatTimeAgo(new Date(Date.now() - 5 * 60 * 1000))}`
                      : "Last ingest failed"}
                </span>
              </div>
              <div class="text-xs text-fg-dim text-right">
                Duration: {formatDuration(history.lastDurationMs)}
              </div>
            </div>

            <div class="grid grid-cols-3 gap-2 text-center">
              <div class="bg-bg p-2 rounded">
                <div class="text-sm font-semibold text-green">{history.successCount}</div>
                <div class="text-2xs text-fg-dim">Successes</div>
              </div>
              <div class="bg-bg p-2 rounded">
                <div class="text-sm font-semibold text-red">{history.failureCount}</div>
                <div class="text-2xs text-fg-dim">Failures</div>
              </div>
              <div class="bg-bg p-2 rounded">
                <div class="text-sm font-semibold text-blue">{history.lastImported}</div>
                <div class="text-2xs text-fg-dim">Last Import</div>
              </div>
            </div>
          </div>
        {:else}
          <div class="border-t border-bg-muted pt-3 mt-3">
            <div class="text-xs text-fg-dim italic">No ingest history available</div>
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

  <div class="mt-4 p-3 bg-bg-soft border border-bg-muted rounded text-xs">
    <div class="flex flex-wrap gap-4">
      <div class="flex items-center gap-1">
        <span class="w-2 h-2 rounded-full bg-green"></span>
        <span class="text-fg-dim">Healthy source</span>
      </div>
      <div class="flex items-center gap-1">
        <span class="w-2 h-2 rounded-full bg-yellow"></span>
        <span class="text-fg-dim">Stale (>5min)</span>
      </div>
      <div class="flex items-center gap-1">
        <span class="w-2 h-2 rounded-full bg-red"></span>
        <span class="text-fg-dim">Failed</span>
      </div>
    </div>
  </div>
</div>
