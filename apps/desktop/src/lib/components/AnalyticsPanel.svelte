<script lang="ts">
  import type {
    ActivityStats,
    CostStats,
    ErrorStats,
    FileLeaderboardEntry,
    LatencyDistribution,
    LongRunningToolCall,
    ModelUsageStats,
    PatchChurnStats,
    ToolFrequencyStats,
  } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";

  type TimeRange = "7d" | "30d" | "90d";

  type Tab = "overview" | "tools" | "files" | "churn" | "latency" | "efficiency";

  let loading = $state(true);
  let error = $state<string | null>(null);
  let timeRange = $state<TimeRange>("30d");
  let activityStats = $state<ActivityStats[]>([]);
  let errorStats = $state<ErrorStats[]>([]);
  let toolFrequency = $state<ToolFrequencyStats[]>([]);
  let filesLeaderboard = $state<FileLeaderboardEntry[]>([]);
  let patchChurn = $state<PatchChurnStats[]>([]);
  let longRunningTools = $state<LongRunningToolCall[]>([]);
  let costStatsBySource = $state<CostStats[]>([]);
  let costStatsByProject = $state<CostStats[]>([]);
  let modelUsageStats = $state<ModelUsageStats[]>([]);
  let latencyDistribution = $state<LatencyDistribution | null>(null);
  let activeSection = $state<Tab>("overview");

  async function loadAllStats() {
    loading = true;
    error = null;

    try {
      const [activity, errors, tools, files, churn, latency, costBySource, costByProject, models, latencyDist] =
        await Promise.all([
          invoke<ActivityStats[]>("get_activity_stats", { since: timeRange, until: null }),
          invoke<ErrorStats[]>("get_error_stats", { since: timeRange, until: null }),
          invoke<ToolFrequencyStats[]>("get_tool_call_frequency", { since: timeRange, until: null }),
          invoke<FileLeaderboardEntry[]>("get_files_leaderboard", { since: timeRange, until: null, limit: 20 }),
          invoke<PatchChurnStats[]>("get_patch_churn", { since: timeRange, until: null }),
          invoke<LongRunningToolCall[]>("get_long_running_tools", {
            since: timeRange,
            until: null,
            min_duration_ms: 5000,
            limit: 20,
          }),
          invoke<CostStats[]>("get_cost_stats_by_source", { source: null, since: timeRange, until: null }),
          invoke<CostStats[]>("get_cost_stats_by_project", { source: null, since: timeRange, until: null }),
          invoke<ModelUsageStats[]>("get_model_usage_stats", { source: null, since: timeRange, until: null }),
          invoke<LatencyDistribution>("get_latency_distribution", { source: null, since: timeRange, until: null }),
        ]);

      activityStats = activity;
      errorStats = errors;
      toolFrequency = tools;
      filesLeaderboard = files;
      patchChurn = churn;
      longRunningTools = latency;
      costStatsBySource = costBySource;
      costStatsByProject = costByProject;
      modelUsageStats = models;
      latencyDistribution = latencyDist;
    } catch (e) {
      error = String(e);
      console.error("Failed to load analytics:", e);
    } finally {
      loading = false;
    }
  }

  const formatCurrency = (cost?: number | null): string => (cost == null ? "N/A" : `$${cost.toFixed(4)}`);

  const getTotalCost = (): number => costStatsBySource.reduce((sum, s) => sum + (s.total_cost || 0), 0);

  const getTotalSessionsWithCost = (): number => costStatsBySource.reduce((sum, s) => sum + s.session_count, 0);

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  }

  const getTotalEvents = (): number => activityStats.reduce((sum, s) => sum + s.event_count, 0);

  const getTotalSessions = (): number => activityStats.reduce((sum, s) => sum + s.session_count, 0);

  const getTotalErrors = (): number => errorStats.reduce((sum, s) => sum + s.error_count, 0);

  const getTotalToolCalls = (): number => toolFrequency.reduce((sum, t) => sum + t.call_count, 0);

  const getTotalLinesChanged = (): { added: number; removed: number } => {
    return patchChurn.reduce(
      (acc, p) => ({ added: acc.added + p.lines_added, removed: acc.removed + p.lines_removed }),
      { added: 0, removed: 0 },
    );
  };

  onMount(() => {
    loadAllStats();
  });
</script>

{#snippet noData(msg: string)}
  <div class="flex items-center justify-center h-64 text-fg-dim" transition:fade>
    <p>{msg}</p>
  </div>
{/snippet}

{#snippet overviewSection()}
  <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
    <div class="p-4 bg-surface border border-surface-muted rounded">
      <div class="text-xs text-fg-dim uppercase tracking-wide mb-1">Total Events</div>
      <div class="text-2xl font-semibold text-fg">{getTotalEvents().toLocaleString()}</div>
    </div>
    <div class="p-4 bg-surface border border-surface-muted rounded">
      <div class="text-xs text-fg-dim uppercase tracking-wide mb-1">Sessions</div>
      <div class="text-2xl font-semibold text-fg">{getTotalSessions().toLocaleString()}</div>
    </div>
    <div class="p-4 bg-surface border border-surface-muted rounded">
      <div class="text-xs text-fg-dim uppercase tracking-wide mb-1">Tool Calls</div>
      <div class="text-2xl font-semibold text-fg">{getTotalToolCalls().toLocaleString()}</div>
    </div>
    <div class="p-4 bg-surface border border-surface-muted rounded">
      <div class="text-xs text-fg-dim uppercase tracking-wide mb-1">Errors</div>
      <div
        class="text-2xl font-semibold"
        class:text-red={getTotalErrors() > 0}
        class:text-green={getTotalErrors() === 0}>
        {getTotalErrors().toLocaleString()}
      </div>
    </div>
  </div>

  {#if activityStats.length > 0}
    <div class="p-4 bg-surface border border-surface-muted rounded mb-4">
      <h3 class="m-0 mb-4 text-sm font-semibold text-fg">Activity Over Time</h3>
      <div class="flex items-end gap-1 h-40 pb-8 relative">
        {#each activityStats as stat}
          {@const maxEvents = Math.max(...activityStats.map((s) => s.event_count))}
          {@const barHeight = maxEvents > 0 ? (stat.event_count / maxEvents) * 100 : 0}
          <div class="flex-1 flex flex-col items-center relative h-full group">
            <div
              class="w-full bg-blue rounded-t-sm min-h-0.5 transition-colors group-hover:bg-blue-bright"
              style="height: {barHeight}%">
            </div>
            <div class="absolute -bottom-6 text-2xs text-fg-dim -rotate-45 origin-center whitespace-nowrap">
              {stat.day.slice(5)}
            </div>
            <div
              class="absolute bottom-full left-1/2 -translate-x-1/2 px-2 py-1 bg-surface border border-surface-muted rounded text-xs text-fg whitespace-nowrap opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 z-10">
              {stat.day}: {stat.event_count} events
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if patchChurn.length > 0}
    {@const totals = getTotalLinesChanged()}
    <div class="p-4 bg-surface border border-surface-muted rounded">
      <h3 class="m-0 mb-3 text-sm font-semibold text-fg">Code Churn Summary</h3>
      <div class="flex gap-8">
        <div>
          <span class="text-xs text-fg-dim">Lines Added</span>
          <div class="text-xl font-semibold text-green">+{totals.added.toLocaleString()}</div>
        </div>
        <div>
          <span class="text-xs text-fg-dim">Lines Removed</span>
          <div class="text-xl font-semibold text-red">-{totals.removed.toLocaleString()}</div>
        </div>
        <div>
          <span class="text-xs text-fg-dim">Net Change</span>
          <div
            class="text-xl font-semibold"
            class:text-green={totals.added > totals.removed}
            class:text-red={totals.added < totals.removed}>
            {totals.added > totals.removed ? "+" : ""}{(totals.added - totals.removed).toLocaleString()}
          </div>
        </div>
      </div>
    </div>
  {/if}
{/snippet}

{#snippet toolsSection()}
  {#if toolFrequency.length > 0}
    <div class="bg-surface border border-surface-muted rounded overflow-hidden">
      <div class="px-4 py-3 border-b border-surface-muted bg-surface-soft">
        <h3 class="m-0 text-sm font-semibold text-fg">Tool Call Frequency</h3>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-surface-muted text-left text-xs text-fg-dim">
              <th class="px-4 py-2 font-medium">Tool</th>
              <th class="px-4 py-2 font-medium text-right">Calls</th>
              <th class="px-4 py-2 font-medium text-right">Sessions</th>
              <th class="px-4 py-2 font-medium text-right">Avg Duration</th>
              <th class="px-4 py-2 font-medium text-right">Max Duration</th>
            </tr>
          </thead>
          <tbody>
            {#each toolFrequency as tool}
              <tr class="border-b border-surface-muted last:border-b-0 hover:bg-surface-soft">
                <td class="px-4 py-3 font-medium text-fg">{tool.tool_name}</td>
                <td class="px-4 py-3 text-right">{tool.call_count.toLocaleString()}</td>
                <td class="px-4 py-3 text-right text-fg-dim">{tool.sessions}</td>
                <td class="px-4 py-3 text-right text-fg-dim">
                  {tool.avg_duration_ms ? formatDuration(tool.avg_duration_ms) : "-"}
                </td>
                <td class="px-4 py-3 text-right text-fg-dim">
                  {tool.max_duration_ms ? formatDuration(tool.max_duration_ms) : "-"}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {:else}
    {@render noData("No tool call data available")}
  {/if}
{/snippet}

{#snippet filesSection()}
  {#if filesLeaderboard.length > 0}
    <div class="bg-surface border border-surface-muted rounded overflow-hidden">
      <div class="px-4 py-3 border-b border-surface-muted bg-surface-soft">
        <h3 class="m-0 text-sm font-semibold text-fg">Files Touched Leaderboard</h3>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-surface-muted text-left text-xs text-fg-dim">
              <th class="px-4 py-2 font-medium">File Path</th>
              <th class="px-4 py-2 font-medium text-right">Touches</th>
              <th class="px-4 py-2 font-medium text-right">Sessions</th>
              <th class="px-4 py-2 font-medium text-right">Lines Added</th>
              <th class="px-4 py-2 font-medium text-right">Lines Removed</th>
            </tr>
          </thead>
          <tbody>
            {#each filesLeaderboard as file}
              <tr class="border-b border-surface-muted last:border-b-0 hover:bg-surface-soft">
                <td class="px-4 py-3 font-mono text-xs text-fg truncate max-w-md" title={file.file_path}>
                  {file.file_path}
                </td>
                <td class="px-4 py-3 text-right font-medium">{file.touch_count}</td>
                <td class="px-4 py-3 text-right text-fg-dim">{file.sessions}</td>
                <td class="px-4 py-3 text-right text-green">+{file.total_lines_added}</td>
                <td class="px-4 py-3 text-right text-red">-{file.total_lines_removed}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {:else}
    {@render noData("No file touch data available")}
  {/if}
{/snippet}

{#snippet churnSection()}
  {#if patchChurn.length > 0}
    <div class="bg-surface border border-surface-muted rounded overflow-hidden">
      <div class="px-4 py-3 border-b border-surface-muted bg-surface-soft">
        <h3 class="m-0 text-sm font-semibold text-fg">Patch Churn by Day</h3>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-surface-muted text-left text-xs text-fg-dim">
              <th class="px-4 py-2 font-medium">Date</th>
              <th class="px-4 py-2 font-medium text-right">Files Changed</th>
              <th class="px-4 py-2 font-medium text-right">Lines Added</th>
              <th class="px-4 py-2 font-medium text-right">Lines Removed</th>
              <th class="px-4 py-2 font-medium text-right">Net Change</th>
              <th class="px-4 py-2 font-medium text-right">Sessions</th>
            </tr>
          </thead>
          <tbody>
            {#each patchChurn as day}
              <tr class="border-b border-surface-muted last:border-b-0 hover:bg-surface-soft">
                <td class="px-4 py-3 text-fg">{day.day}</td>
                <td class="px-4 py-3 text-right">{day.files_changed}</td>
                <td class="px-4 py-3 text-right text-green">+{day.lines_added}</td>
                <td class="px-4 py-3 text-right text-red">-{day.lines_removed}</td>
                <td
                  class="px-4 py-3 text-right font-medium"
                  class:text-green={day.lines_added > day.lines_removed}
                  class:text-red={day.lines_added < day.lines_removed}>
                  {day.lines_added > day.lines_removed ? "+" : ""}{day.lines_added - day.lines_removed}
                </td>
                <td class="px-4 py-3 text-right text-fg-dim">{day.sessions}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {:else}
    {@render noData("No patch churn data available")}
  {/if}
{/snippet}

{#snippet latencySection()}
  {#if longRunningTools.length > 0}
    <div class="bg-surface border border-surface-muted rounded overflow-hidden">
      <div class="px-4 py-3 border-b border-surface-muted bg-surface-soft">
        <h3 class="m-0 text-sm font-semibold text-fg">Long-Running Tool Calls (5s+)</h3>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-surface-muted text-left text-xs text-fg-dim">
              <th class="px-4 py-2 font-medium">Tool</th>
              <th class="px-4 py-2 font-medium text-right">Duration</th>
              <th class="px-4 py-2 font-medium">Started</th>
              <th class="px-4 py-2 font-medium">Session</th>
              <th class="px-4 py-2 font-medium">Project</th>
              <th class="px-4 py-2 font-medium">Status</th>
            </tr>
          </thead>
          <tbody>
            {#each longRunningTools as tool}
              <tr class="border-b border-surface-muted last:border-b-0 hover:bg-surface-soft">
                <td class="px-4 py-3 font-medium text-fg">{tool.tool_name}</td>
                <td class="px-4 py-3 text-right">{formatDuration(tool.duration_ms)}</td>
                <td class="px-4 py-3 text-fg-dim text-xs">{tool.started_at}</td>
                <td class="px-4 py-3 font-mono text-xs text-fg-dim">{tool.session_external_id.slice(0, 8)}...</td>
                <td class="px-4 py-3 text-fg-dim text-xs">{tool.project || "-"}</td>
                <td class="px-4 py-3">
                  {#if tool.error_message}
                    <span class="text-xs px-2 py-0.5 bg-red text-surface rounded">ERROR</span>
                  {:else}
                    <span class="text-xs px-2 py-0.5 bg-green text-surface rounded">OK</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {:else}
    {@render noData("No long-running tool calls found")}
  {/if}
{/snippet}

{#snippet efficiencySection()}
  {#if costStatsBySource.length > 0 || costStatsByProject.length > 0}
    <div class="space-y-4">
      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <div class="p-4 bg-surface border border-surface-muted rounded">
          <div class="text-xs text-fg-dim uppercase tracking-wide mb-1">Total Cost</div>
          <div class="text-2xl font-semibold text-fg">{formatCurrency(getTotalCost())}</div>
        </div>
        <div class="p-4 bg-surface border border-surface-muted rounded">
          <div class="text-xs text-fg-dim uppercase tracking-wide mb-1">Avg Cost/Session</div>
          <div class="text-2xl font-semibold text-fg">
            {formatCurrency(getTotalCost() / getTotalSessionsWithCost() || 0)}
          </div>
        </div>
        <div class="p-4 bg-surface border border-surface-muted rounded">
          <div class="text-xs text-fg-dim uppercase tracking-wide mb-1">P50 Latency</div>
          <div class="text-2xl font-semibold text-fg">
            {latencyDistribution?.p50_latency ? formatDuration(latencyDistribution.p50_latency) : "N/A"}
          </div>
        </div>
        <div class="p-4 bg-surface border border-surface-muted rounded">
          <div class="text-xs text-fg-dim uppercase tracking-wide mb-1">P95 Latency</div>
          <div class="text-2xl font-semibold text-fg">
            {latencyDistribution?.p95_latency ? formatDuration(latencyDistribution.p95_latency) : "N/A"}
          </div>
        </div>
      </div>

      {#if costStatsBySource.length > 0}
        <div class="bg-surface border border-surface-muted rounded overflow-hidden">
          <div class="px-4 py-3 border-b border-surface-muted bg-surface-soft">
            <h3 class="m-0 text-sm font-semibold text-fg">Cost by Source</h3>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-surface-muted text-left text-xs text-fg-dim">
                  <th class="px-4 py-2 font-medium">Source</th>
                  <th class="px-4 py-2 font-medium text-right">Sessions</th>
                  <th class="px-4 py-2 font-medium text-right">Total Cost</th>
                  <th class="px-4 py-2 font-medium text-right">Avg Cost/Session</th>
                  <th class="px-4 py-2 font-medium text-right">Input Tokens</th>
                  <th class="px-4 py-2 font-medium text-right">Output Tokens</th>
                </tr>
              </thead>
              <tbody>
                {#each costStatsBySource as stat}
                  <tr class="border-b border-surface-muted last:border-b-0 hover:bg-surface-soft">
                    <td class="px-4 py-3 font-medium text-fg">{stat.dimension}</td>
                    <td class="px-4 py-3 text-right">{stat.session_count}</td>
                    <td class="px-4 py-3 text-right font-semibold">{formatCurrency(stat.total_cost)}</td>
                    <td class="px-4 py-3 text-right text-fg-dim">{formatCurrency(stat.avg_cost_per_session)}</td>
                    <td class="px-4 py-3 text-right text-fg-dim">
                      {stat.total_input_tokens ? (stat.total_input_tokens / 1000).toFixed(1) + "k" : "N/A"}
                    </td>
                    <td class="px-4 py-3 text-right text-fg-dim">
                      {stat.total_output_tokens ? (stat.total_output_tokens / 1000).toFixed(1) + "k" : "N/A"}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      {#if costStatsByProject.length > 0}
        <div class="bg-surface border border-surface-muted rounded overflow-hidden">
          <div class="px-4 py-3 border-b border-surface-muted bg-surface-soft">
            <h3 class="m-0 text-sm font-semibold text-fg">Cost by Project</h3>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-surface-muted text-left text-xs text-fg-dim">
                  <th class="px-4 py-2 font-medium">Project</th>
                  <th class="px-4 py-2 font-medium text-right">Sessions</th>
                  <th class="px-4 py-2 font-medium text-right">Total Cost</th>
                  <th class="px-4 py-2 font-medium text-right">Avg Cost/Session</th>
                </tr>
              </thead>
              <tbody>
                {#each costStatsByProject.slice(0, 10) as stat}
                  <tr class="border-b border-surface-muted last:border-b-0 hover:bg-surface-soft">
                    <td class="px-4 py-3 font-medium text-fg">{stat.dimension}</td>
                    <td class="px-4 py-3 text-right">{stat.session_count}</td>
                    <td class="px-4 py-3 text-right font-semibold">{formatCurrency(stat.total_cost)}</td>
                    <td class="px-4 py-3 text-right text-fg-dim">{formatCurrency(stat.avg_cost_per_session)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
            {#if costStatsByProject.length > 10}
              <div class="px-4 py-2 text-xs text-fg-dim text-center border-t border-surface-muted">
                ... and {costStatsByProject.length - 10} more projects
              </div>
            {/if}
          </div>
        </div>
      {/if}

      {#if modelUsageStats.length > 0}
        <div class="bg-surface border border-surface-muted rounded overflow-hidden">
          <div class="px-4 py-3 border-b border-surface-muted bg-surface-soft">
            <h3 class="m-0 text-sm font-semibold text-fg">Model Usage</h3>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b border-surface-muted text-left text-xs text-fg-dim">
                  <th class="px-4 py-2 font-medium">Model/Provider</th>
                  <th class="px-4 py-2 font-medium text-right">Sessions</th>
                  <th class="px-4 py-2 font-medium text-right">Total Cost</th>
                  <th class="px-4 py-2 font-medium text-right">Input Tokens</th>
                  <th class="px-4 py-2 font-medium text-right">Output Tokens</th>
                  <th class="px-4 py-2 font-medium text-right">Avg Latency</th>
                </tr>
              </thead>
              <tbody>
                {#each modelUsageStats as stat}
                  <tr class="border-b border-surface-muted last:border-b-0 hover:bg-surface-soft">
                    <td class="px-4 py-3 font-medium text-fg">{stat.provider}/{stat.model}</td>
                    <td class="px-4 py-3 text-right">{stat.session_count}</td>
                    <td class="px-4 py-3 text-right font-semibold">{formatCurrency(stat.total_cost)}</td>
                    <td class="px-4 py-3 text-right text-fg-dim">
                      {stat.total_input_tokens ? (stat.total_input_tokens / 1000).toFixed(1) + "k" : "N/A"}
                    </td>
                    <td class="px-4 py-3 text-right text-fg-dim">
                      {stat.total_output_tokens ? (stat.total_output_tokens / 1000).toFixed(1) + "k" : "N/A"}
                    </td>
                    <td class="px-4 py-3 text-right text-fg-dim">
                      {stat.avg_latency_ms ? formatDuration(stat.avg_latency_ms) : "N/A"}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}
    </div>
  {:else}
    {@render noData("No cost data available")}
  {/if}
{/snippet}

<div class="flex flex-col h-full overflow-hidden bg-surface-soft">
  <div class="p-4 border-b border-surface-muted bg-surface">
    <div class="flex justify-between items-center mb-4">
      <h2 class="m-0 text-lg font-semibold text-fg">Analytics Dashboard</h2>
      <div class="flex gap-2">
        <select
          class="px-3 py-1.5 bg-surface-soft border border-surface-muted rounded text-fg font-inherit text-sm cursor-pointer focus:outline-none focus:border-blue"
          value={timeRange}
          onchange={(e) => {
            timeRange = e.currentTarget.value as TimeRange;
            loadAllStats();
          }}>
          <option value="7d">Last 7 days</option>
          <option value="30d">Last 30 days</option>
          <option value="90d">Last 90 days</option>
        </select>
        <button
          class="px-3 py-1.5 bg-blue text-surface border-none rounded font-inherit text-sm cursor-pointer transition-colors hover:bg-blue-bright disabled:opacity-50"
          onclick={loadAllStats}
          disabled={loading}>
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>
    </div>

    <div class="flex gap-1">
      {#each ["overview", "tools", "files", "churn", "latency", "efficiency"] as section}
        <button
          class="px-4 py-2 bg-transparent border-none text-fg-dim font-inherit text-sm cursor-pointer transition-all rounded hover:bg-surface-soft"
          class:bg-blue={activeSection === section}
          class:text-surface={activeSection === section}
          class:hover:bg-blue-bright={activeSection === section}
          onclick={() => (activeSection = section as Tab)}>
          {section.charAt(0).toUpperCase() + section.slice(1)}
        </button>
      {/each}
    </div>
  </div>

  {#if error}
    <div class="mx-4 my-2 p-3 bg-red text-surface rounded text-sm" transition:fade>{error}</div>
  {/if}

  <div class="flex-1 overflow-y-auto p-4">
    {#if loading && activityStats.length === 0}
      <div class="flex items-center justify-center h-full text-fg-dim">
        <div class="text-center">
          <div class="i-ri-loader-4-line text-3xl mb-3 animate-spin"></div>
          <p>Loading analytics...</p>
        </div>
      </div>
    {:else if activeSection === "overview"}
      {@render overviewSection()}
    {:else if activeSection === "tools"}
      {@render toolsSection()}
    {:else if activeSection === "files"}
      {@render filesSection()}
    {:else if activeSection === "churn"}
      {@render churnSection()}
    {:else if activeSection === "latency"}
      {@render latencySection()}
    {:else if activeSection === "efficiency"}
      {@render efficiencySection()}
    {/if}
  </div>
</div>
