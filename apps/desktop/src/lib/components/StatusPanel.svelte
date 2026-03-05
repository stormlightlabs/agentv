<script lang="ts">
  import type { SourceHealth } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";

  type Props = { onRefresh?: () => void };

  let { onRefresh }: Props = $props();

  let healthData = $state<SourceHealth[]>([]);
  let loading = $state(false);
  let lastChecked = $state<Date | null>(null);

  async function loadHealth() {
    loading = true;
    try {
      healthData = await invoke<SourceHealth[]>("get_source_health");
      lastChecked = new Date();
      onRefresh?.();
    } catch (error) {
      console.error("Failed to load source health:", error);
    } finally {
      loading = false;
    }
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

  onMount(() => {
    loadHealth();
  });
</script>

<div class="flex h-full flex-col overflow-hidden p-4" in:fade={{ duration: 200 }}>
  <div class="mb-4 flex items-center justify-between">
    <h2 class="text-fg m-0 text-lg font-semibold">Data Source Status</h2>
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

  <div class="space-y-3">
    {#each healthData as source, index (source.source)}
      <div
        class="bg-surface-soft border-surface-muted hover:border-blue rounded border p-3 transition-all"
        in:fly={{ y: 10, duration: 200, delay: index * 50 }}>
        <div class="mb-2 flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="text-fg text-sm font-medium capitalize">
              {source.source}
            </span>
            <span class="text-surface rounded px-1.5 py-0.5 text-xs uppercase {getStatusColor(source.status)}">
              {source.status}
            </span>
          </div>
          <span class="{getStatusIcon(source.status)} {getStatusColor(source.status).replace('bg-', 'text-')}"></span>
        </div>

        {#if source.path}
          <div class="text-fg-dim mb-1 flex items-start gap-1 text-xs">
            <span class="i-ri-folder-line mt-0.5 shrink-0"></span>
            <span class="break-all">{source.path}</span>
          </div>
        {/if}

        {#if source.message}
          <div class="text-fg-dim flex items-start gap-1 text-xs">
            <span class="i-ri-information-line mt-0.5 shrink-0"></span>
            <span class="break-all">{source.message}</span>
          </div>
        {/if}
      </div>
    {:else}
      {#if !loading}
        <div class="text-center text-fg-dim py-8" in:fade>
          <div class="i-ri-error-warning-line text-3xl mb-2 mx-auto"></div>
          <p class="text-sm">No health data available</p>
        </div>
      {/if}
    {/each}
  </div>

  <div class="bg-surface-soft border-surface-muted mt-4 rounded border p-3">
    <h3 class="text-fg-dim mb-2 text-xs font-semibold tracking-wide uppercase">Legend</h3>
    <div class="space-y-1 text-xs">
      <div class="flex items-center gap-2">
        <span class="bg-green h-2 w-2 rounded-full"></span>
        <span class="text-fg-dim">Healthy - Source is accessible and working</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="bg-yellow h-2 w-2 rounded-full"></span>
        <span class="text-fg-dim">Degraded - Source accessible but may have issues</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="bg-red h-2 w-2 rounded-full"></span>
        <span class="text-fg-dim">Unhealthy - Source is not accessible</span>
      </div>
      <div class="flex items-center gap-2">
        <span class="bg-fg-dim h-2 w-2 rounded-full"></span>
        <span class="text-fg-dim">Unknown - Could not determine status</span>
      </div>
    </div>
  </div>
</div>
