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
    } catch (error_) {
      error = String(error_);
      console.error("Failed to load session metrics:", error_);
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

<div class="bg-surface-soft border-surface-muted border-b">
  {#if loading}
    <div class="text-fg-dim flex items-center gap-2 px-6 py-3 text-sm">
      <span class="i-ri-loader-4-line animate-spin"></span>
      Loading cost & latency metrics...
    </div>
  {:else if error}
    <div class="text-red px-6 py-3 text-sm" transition:fade>
      Failed to load metrics: {error}
    </div>
  {:else if metrics}
    {@const hasCostData =
      metrics.estimated_cost != null || metrics.input_tokens != null || metrics.output_tokens != null}
    {@const hasLatencyData =
      metrics.avg_latency_ms != null || metrics.p50_latency_ms != null || metrics.p95_latency_ms != null}

    <div class="px-6 py-4" transition:fade>
      <div class="mb-3 flex items-center justify-between">
        <h3 class="text-fg m-0 flex items-center gap-2 text-sm font-semibold">
          <span class="i-ri-coins-line text-fg-dim"></span>
          Cost & Efficiency
        </h3>
        {#if metrics.model}
          <span class="text-fg-dim bg-surface-muted rounded px-2 py-0.5 text-xs">
            {metrics.provider || "unknown"}/{metrics.model}
          </span>
        {/if}
      </div>

      <div class="grid grid-cols-2 gap-4 lg:grid-cols-4">
        {#if hasCostData}
          <div class="bg-surface border-surface-muted rounded border p-3">
            <div class="text-fg-dim mb-1 text-xs">Estimated Cost</div>
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

          <div class="bg-surface border-surface-muted rounded border p-3">
            <div class="text-fg-dim mb-1 text-xs">Input Tokens</div>
            <div class="text-fg text-xl font-semibold">
              {formatTokens(metrics.input_tokens)}
            </div>
          </div>

          <div class="bg-surface border-surface-muted rounded border p-3">
            <div class="text-fg-dim mb-1 text-xs">Output Tokens</div>
            <div class="text-fg text-xl font-semibold">
              {formatTokens(metrics.output_tokens)}
            </div>
          </div>
        {:else}
          <div class="bg-surface border-surface-muted col-span-2 rounded border p-3 lg:col-span-3">
            <div class="text-fg-dim text-sm">
              No cost data available. Cost tracking requires model metadata and token estimates.
            </div>
          </div>
        {/if}

        {#if hasLatencyData}
          <div class="bg-surface border-surface-muted rounded border p-3">
            <div class="text-fg-dim mb-1 text-xs">Avg Latency</div>
            <div class="text-fg text-xl font-semibold">
              {formatDuration(metrics.avg_latency_ms)}
            </div>
          </div>

          <div class="bg-surface border-surface-muted rounded border p-3">
            <div class="text-fg-dim mb-1 text-xs">P50 Latency</div>
            <div class="text-fg text-xl font-semibold">
              {formatDuration(metrics.p50_latency_ms)}
            </div>
          </div>

          <div class="bg-surface border-surface-muted rounded border p-3">
            <div class="text-fg-dim mb-1 text-xs">P95 Latency</div>
            <div
              class="text-xl font-semibold"
              class:text-green={metrics.p95_latency_ms && metrics.p95_latency_ms < 5000}
              class:text-yellow={metrics.p95_latency_ms &&
                metrics.p95_latency_ms >= 5000 &&
                metrics.p95_latency_ms < 15_000}
              class:text-red={metrics.p95_latency_ms && metrics.p95_latency_ms >= 15_000}>
              {formatDuration(metrics.p95_latency_ms)}
            </div>
          </div>

          <div class="bg-surface border-surface-muted rounded border p-3">
            <div class="text-fg-dim mb-1 text-xs">Duration</div>
            <div class="text-fg text-xl font-semibold">
              {#if metrics.duration_seconds}
                {Math.round(metrics.duration_seconds / 60)}m
              {:else}
                N/A
              {/if}
            </div>
          </div>
        {:else}
          <div class="bg-surface border-surface-muted rounded border p-3">
            <div class="text-fg-dim text-sm">No latency data available</div>
          </div>
        {/if}
      </div>

      {#if metrics.tool_call_count > 0 || metrics.error_count > 0}
        <div class="border-surface-muted mt-4 border-t pt-3">
          <div class="grid grid-cols-3 gap-4 text-sm">
            <div class="flex items-center gap-2">
              <span class="i-ri-tools-line text-fg-dim"></span>
              <span class="text-fg-dim">Tool Calls:</span>
              <span class="text-fg font-semibold">{metrics.tool_call_count}</span>
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
              <span class="text-fg font-semibold">{metrics.files_touched}</span>
            </div>
          </div>
        </div>
      {/if}
    </div>
  {:else}
    <div class="text-fg-dim px-6 py-3 text-sm" transition:fade>No metrics available for this session.</div>
  {/if}
</div>
