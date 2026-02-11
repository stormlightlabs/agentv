<script lang="ts">
  import type { SessionData, SessionMetricsData } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";

  type Props = { session: SessionData };

  let { session }: Props = $props();

  let metrics = $state<SessionMetricsData | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    await loadMetrics();
  });

  async function loadMetrics() {
    loading = true;
    error = null;
    try {
      metrics = await invoke<SessionMetricsData | null>("get_session_metrics", { sessionId: session.id });
    } catch (e) {
      error = String(e);
      console.error("Failed to load session metrics:", e);
    } finally {
      loading = false;
    }
  }

  function formatCurrency(cost: number | null | undefined): string {
    if (cost == null || cost === undefined) return "N/A";
    return `$${cost.toFixed(4)}`;
  }

  function formatDuration(ms: number | null | undefined): string {
    if (ms == null || ms === undefined) return "N/A";
    if (ms < 1000) return `${Math.round(ms)}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  }

  function formatTokens(tokens: number | null | undefined): string {
    if (tokens == null || tokens === undefined) return "N/A";
    if (tokens >= 1000) {
      return `${(tokens / 1000).toFixed(1)}k`;
    }
    return tokens.toString();
  }
</script>

<div class="bg-surface-soft border-b border-surface-muted">
  {#if loading}
    <div class="px-6 py-3 flex items-center gap-2 text-fg-dim text-sm">
      <span class="i-ri-loader-4-line animate-spin"></span>
      Loading cost & latency metrics...
    </div>
  {:else if error}
    <div class="px-6 py-3 text-red text-sm" transition:fade>
      Failed to load metrics: {error}
    </div>
  {:else if metrics}
    {@const hasCostData =
      metrics.estimated_cost != null || metrics.input_tokens != null || metrics.output_tokens != null}
    {@const hasLatencyData =
      metrics.avg_latency_ms != null || metrics.p50_latency_ms != null || metrics.p95_latency_ms != null}

    <div class="px-6 py-4" transition:fade>
      <div class="flex items-center justify-between mb-3">
        <h3 class="text-sm font-semibold text-fg flex items-center gap-2 m-0">
          <span class="i-ri-coins-line text-fg-dim"></span>
          Cost & Efficiency
        </h3>
        {#if metrics.model}
          <span class="text-xs text-fg-dim bg-surface-muted px-2 py-0.5 rounded">
            {metrics.provider || "unknown"}/{metrics.model}
          </span>
        {/if}
      </div>

      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <!-- Cost Column -->
        {#if hasCostData}
          <div class="p-3 bg-surface rounded border border-surface-muted">
            <div class="text-xs text-fg-dim mb-1">Estimated Cost</div>
            <div
              class="text-xl font-semibold"
              class:text-green={metrics.estimated_cost && metrics.estimated_cost < 0.01}
              class:text-yellow={metrics.estimated_cost &&
                metrics.estimated_cost >= 0.01 &&
                metrics.estimated_cost < 0.1}
              class:text-red={metrics.estimated_cost && metrics.estimated_cost >= 0.1}>
              {formatCurrency(metrics.estimated_cost)}
            </div>
          </div>

          <div class="p-3 bg-surface rounded border border-surface-muted">
            <div class="text-xs text-fg-dim mb-1">Input Tokens</div>
            <div class="text-xl font-semibold text-fg">
              {formatTokens(metrics.input_tokens)}
            </div>
          </div>

          <div class="p-3 bg-surface rounded border border-surface-muted">
            <div class="text-xs text-fg-dim mb-1">Output Tokens</div>
            <div class="text-xl font-semibold text-fg">
              {formatTokens(metrics.output_tokens)}
            </div>
          </div>
        {:else}
          <div class="p-3 bg-surface rounded border border-surface-muted col-span-2 lg:col-span-3">
            <div class="text-sm text-fg-dim">
              No cost data available. Cost tracking requires model metadata and token estimates.
            </div>
          </div>
        {/if}

        <!-- Latency Column -->
        {#if hasLatencyData}
          <div class="p-3 bg-surface rounded border border-surface-muted">
            <div class="text-xs text-fg-dim mb-1">Avg Latency</div>
            <div class="text-xl font-semibold text-fg">
              {formatDuration(metrics.avg_latency_ms)}
            </div>
          </div>

          <div class="p-3 bg-surface rounded border border-surface-muted">
            <div class="text-xs text-fg-dim mb-1">P50 Latency</div>
            <div class="text-xl font-semibold text-fg">
              {formatDuration(metrics.p50_latency_ms)}
            </div>
          </div>

          <div class="p-3 bg-surface rounded border border-surface-muted">
            <div class="text-xs text-fg-dim mb-1">P95 Latency</div>
            <div
              class="text-xl font-semibold"
              class:text-green={metrics.p95_latency_ms && metrics.p95_latency_ms < 5000}
              class:text-yellow={metrics.p95_latency_ms &&
                metrics.p95_latency_ms >= 5000 &&
                metrics.p95_latency_ms < 15000}
              class:text-red={metrics.p95_latency_ms && metrics.p95_latency_ms >= 15000}>
              {formatDuration(metrics.p95_latency_ms)}
            </div>
          </div>

          <div class="p-3 bg-surface rounded border border-surface-muted">
            <div class="text-xs text-fg-dim mb-1">Duration</div>
            <div class="text-xl font-semibold text-fg">
              {#if metrics.duration_seconds}
                {Math.round(metrics.duration_seconds / 60)}m
              {:else}
                N/A
              {/if}
            </div>
          </div>
        {:else}
          <div class="p-3 bg-surface rounded border border-surface-muted">
            <div class="text-sm text-fg-dim">No latency data available</div>
          </div>
        {/if}
      </div>

      <!-- Efficiency Metrics -->
      {#if metrics.tool_call_count > 0 || metrics.error_count > 0}
        <div class="mt-4 pt-3 border-t border-surface-muted">
          <div class="grid grid-cols-3 gap-4 text-sm">
            <div class="flex items-center gap-2">
              <span class="i-ri-tools-line text-fg-dim"></span>
              <span class="text-fg-dim">Tool Calls:</span>
              <span class="font-semibold text-fg">{metrics.tool_call_count}</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="i-ri-error-warning-line text-fg-dim"></span>
              <span class="text-fg-dim">Errors:</span>
              <span
                class="font-semibold"
                class:text-green={metrics.error_count === 0}
                class:text-red={metrics.error_count > 0}>
                {metrics.error_count}
              </span>
            </div>
            <div class="flex items-center gap-2">
              <span class="i-ri-file-line text-fg-dim"></span>
              <span class="text-fg-dim">Files Touched:</span>
              <span class="font-semibold text-fg">{metrics.files_touched}</span>
            </div>
          </div>
        </div>
      {/if}
    </div>
  {:else}
    <div class="px-6 py-3 text-fg-dim text-sm" transition:fade>No metrics available for this session.</div>
  {/if}
</div>
