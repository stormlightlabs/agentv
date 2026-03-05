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
  import { getDisplayExternalId, getDisplayProject } from "$lib/utils/sessionDisplay";
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
    } catch (error_) {
      error = String(error_);
      console.error("Failed to load analytics:", error_);
    } finally {
      loading = false;
    }
  }

  const formatCurrency = (cost?: number | null): string => (cost == null ? "N/A" : `$${cost.toFixed(4)}`);

  const getTotalCost = (): number => costStatsBySource.reduce((sum, s) => sum + (s.total_cost || 0), 0);

  const getTotalSessionsWithCost = (): number => costStatsBySource.reduce((sum, s) => sum + s.session_count, 0);

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60_000).toFixed(1)}m`;
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
  <div class="text-fg-dim flex h-64 items-center justify-center" transition:fade>
    <p>{msg}</p>
  </div>
{/snippet}

{#snippet overviewSection()}
  <div class="mb-6 grid grid-cols-2 gap-4 lg:grid-cols-4">
    <div class="bg-surface border-surface-muted rounded border p-4">
      <div class="text-fg-dim mb-1 text-xs tracking-wide uppercase">Total Events</div>
      <div class="text-fg text-2xl font-semibold">{getTotalEvents().toLocaleString()}</div>
    </div>
    <div class="bg-surface border-surface-muted rounded border p-4">
      <div class="text-fg-dim mb-1 text-xs tracking-wide uppercase">Sessions</div>
      <div class="text-fg text-2xl font-semibold">{getTotalSessions().toLocaleString()}</div>
    </div>
    <div class="bg-surface border-surface-muted rounded border p-4">
      <div class="text-fg-dim mb-1 text-xs tracking-wide uppercase">Tool Calls</div>
      <div class="text-fg text-2xl font-semibold">{getTotalToolCalls().toLocaleString()}</div>
    </div>
    <div class="bg-surface border-surface-muted rounded border p-4">
      <div class="text-fg-dim mb-1 text-xs tracking-wide uppercase">Errors</div>
      <div
        class="text-2xl font-semibold"
        class:text-red={getTotalErrors() > 0}
        class:text-green={getTotalErrors() === 0}>
        {getTotalErrors().toLocaleString()}
      </div>
    </div>
  </div>

  {#if activityStats.length > 0}
    <div class="bg-surface border-surface-muted mb-4 rounded border p-4">
      <h3 class="text-fg m-0 mb-4 text-sm font-semibold">Activity Over Time</h3>
      <div class="relative flex h-40 items-end gap-1 pb-8">
        {#each activityStats as stat (stat.day)}
          {@const maxEvents = Math.max(...activityStats.map((s) => s.event_count))}
          {@const barHeight = maxEvents > 0 ? (stat.event_count / maxEvents) * 100 : 0}
          <div class="group relative flex h-full flex-1 flex-col items-center">
            <div
              class="bg-blue group-hover:bg-blue-bright min-h-0.5 w-full rounded-t-sm transition-colors"
              style="height: {barHeight}%">
            </div>
            <div class="text-fg-dim absolute -bottom-6 origin-center -rotate-45 text-xs whitespace-nowrap">
              {stat.day.slice(5)}
            </div>
            <div
              class="bg-surface border-surface-muted text-fg pointer-events-none absolute bottom-full left-1/2 z-10 -translate-x-1/2 rounded border px-2 py-1 text-xs whitespace-nowrap opacity-0 transition-opacity group-hover:opacity-100">
              {stat.day}: {stat.event_count} events
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if patchChurn.length > 0}
    {@const totals = getTotalLinesChanged()}
    <div class="bg-surface border-surface-muted rounded border p-4">
      <h3 class="text-fg m-0 mb-3 text-sm font-semibold">Code Churn Summary</h3>
      <div class="flex gap-8">
        <div>
          <span class="text-fg-dim text-xs">Lines Added</span>
          <div class="text-green text-xl font-semibold">+{totals.added.toLocaleString()}</div>
        </div>
        <div>
          <span class="text-fg-dim text-xs">Lines Removed</span>
          <div class="text-red text-xl font-semibold">-{totals.removed.toLocaleString()}</div>
        </div>
        <div>
          <span class="text-fg-dim text-xs">Net Change</span>
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
    <div class="bg-surface border-surface-muted overflow-hidden rounded border">
      <div class="border-surface-muted bg-surface-soft border-b px-4 py-3">
        <h3 class="text-fg m-0 text-sm font-semibold">Tool Call Frequency</h3>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-surface-muted text-fg-dim border-b text-left text-xs">
              <th class="px-4 py-2 font-medium">Tool</th>
              <th class="px-4 py-2 text-right font-medium">Calls</th>
              <th class="px-4 py-2 text-right font-medium">Sessions</th>
              <th class="px-4 py-2 text-right font-medium">Avg Duration</th>
              <th class="px-4 py-2 text-right font-medium">Max Duration</th>
            </tr>
          </thead>
          <tbody>
            {#each toolFrequency as tool (tool.tool_name)}
              <tr class="border-surface-muted hover:bg-surface-soft border-b last:border-b-0">
                <td class="text-fg px-4 py-3 font-medium">{tool.tool_name}</td>
                <td class="px-4 py-3 text-right">{tool.call_count.toLocaleString()}</td>
                <td class="text-fg-dim px-4 py-3 text-right">{tool.sessions}</td>
                <td class="text-fg-dim px-4 py-3 text-right">
                  {tool.avg_duration_ms ? formatDuration(tool.avg_duration_ms) : "-"}
                </td>
                <td class="text-fg-dim px-4 py-3 text-right">
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
    <div class="bg-surface border-surface-muted overflow-hidden rounded border">
      <div class="border-surface-muted bg-surface-soft border-b px-4 py-3">
        <h3 class="text-fg m-0 text-sm font-semibold">Files Touched Leaderboard</h3>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-surface-muted text-fg-dim border-b text-left text-xs">
              <th class="px-4 py-2 font-medium">File Path</th>
              <th class="px-4 py-2 text-right font-medium">Touches</th>
              <th class="px-4 py-2 text-right font-medium">Sessions</th>
              <th class="px-4 py-2 text-right font-medium">Lines Added</th>
              <th class="px-4 py-2 text-right font-medium">Lines Removed</th>
            </tr>
          </thead>
          <tbody>
            {#each filesLeaderboard as file (file.file_path)}
              <tr class="border-surface-muted hover:bg-surface-soft border-b last:border-b-0">
                <td class="text-fg max-w-md truncate px-4 py-3 font-mono text-xs" title={file.file_path}>
                  {file.file_path}
                </td>
                <td class="px-4 py-3 text-right font-medium">{file.touch_count}</td>
                <td class="text-fg-dim px-4 py-3 text-right">{file.sessions}</td>
                <td class="text-green px-4 py-3 text-right">+{file.total_lines_added}</td>
                <td class="text-red px-4 py-3 text-right">-{file.total_lines_removed}</td>
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
    <div class="bg-surface border-surface-muted overflow-hidden rounded border">
      <div class="border-surface-muted bg-surface-soft border-b px-4 py-3">
        <h3 class="text-fg m-0 text-sm font-semibold">Patch Churn by Day</h3>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-surface-muted text-fg-dim border-b text-left text-xs">
              <th class="px-4 py-2 font-medium">Date</th>
              <th class="px-4 py-2 text-right font-medium">Files Changed</th>
              <th class="px-4 py-2 text-right font-medium">Lines Added</th>
              <th class="px-4 py-2 text-right font-medium">Lines Removed</th>
              <th class="px-4 py-2 text-right font-medium">Net Change</th>
              <th class="px-4 py-2 text-right font-medium">Sessions</th>
            </tr>
          </thead>
          <tbody>
            {#each patchChurn as day (day.day)}
              <tr class="border-surface-muted hover:bg-surface-soft border-b last:border-b-0">
                <td class="text-fg px-4 py-3">{day.day}</td>
                <td class="px-4 py-3 text-right">{day.files_changed}</td>
                <td class="text-green px-4 py-3 text-right">+{day.lines_added}</td>
                <td class="text-red px-4 py-3 text-right">-{day.lines_removed}</td>
                <td
                  class="px-4 py-3 text-right font-medium"
                  class:text-green={day.lines_added > day.lines_removed}
                  class:text-red={day.lines_added < day.lines_removed}>
                  {day.lines_added > day.lines_removed ? "+" : ""}{day.lines_added - day.lines_removed}
                </td>
                <td class="text-fg-dim px-4 py-3 text-right">{day.sessions}</td>
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
    <div class="bg-surface border-surface-muted overflow-hidden rounded border">
      <div class="border-surface-muted bg-surface-soft border-b px-4 py-3">
        <h3 class="text-fg m-0 text-sm font-semibold">Long-Running Tool Calls (5s+)</h3>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-surface-muted text-fg-dim border-b text-left text-xs">
              <th class="px-4 py-2 font-medium">Tool</th>
              <th class="px-4 py-2 text-right font-medium">Duration</th>
              <th class="px-4 py-2 font-medium">Started</th>
              <th class="px-4 py-2 font-medium">Session</th>
              <th class="px-4 py-2 font-medium">Project</th>
              <th class="px-4 py-2 font-medium">Status</th>
            </tr>
          </thead>
          <tbody>
            {#each longRunningTools as tool (tool.tool_name)}
              <tr class="border-surface-muted hover:bg-surface-soft border-b last:border-b-0">
                <td class="text-fg px-4 py-3 font-medium">{tool.tool_name}</td>
                <td class="px-4 py-3 text-right">{formatDuration(tool.duration_ms)}</td>
                <td class="text-fg-dim px-4 py-3 text-xs">{tool.started_at}</td>
                <td class="text-fg-dim px-4 py-3 font-mono text-xs">
                  {getDisplayExternalId("unknown", tool.session_external_id)}
                </td>
                <td class="text-fg-dim px-4 py-3 text-xs">{getDisplayProject(tool.project)}</td>
                <td class="px-4 py-3">
                  {#if tool.error_message}
                    <span class="bg-red text-surface rounded px-2 py-0.5 text-xs">ERROR</span>
                  {:else}
                    <span class="bg-green text-surface rounded px-2 py-0.5 text-xs">OK</span>
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
      <div class="mb-6 grid grid-cols-2 gap-4 lg:grid-cols-4">
        <div class="bg-surface border-surface-muted rounded border p-4">
          <div class="text-fg-dim mb-1 text-xs tracking-wide uppercase">Total Cost</div>
          <div class="text-fg text-2xl font-semibold">{formatCurrency(getTotalCost())}</div>
        </div>
        <div class="bg-surface border-surface-muted rounded border p-4">
          <div class="text-fg-dim mb-1 text-xs tracking-wide uppercase">Avg Cost/Session</div>
          <div class="text-fg text-2xl font-semibold">
            {formatCurrency(getTotalCost() / getTotalSessionsWithCost() || 0)}
          </div>
        </div>
        <div class="bg-surface border-surface-muted rounded border p-4">
          <div class="text-fg-dim mb-1 text-xs tracking-wide uppercase">P50 Latency</div>
          <div class="text-fg text-2xl font-semibold">
            {latencyDistribution?.p50_latency ? formatDuration(latencyDistribution.p50_latency) : "N/A"}
          </div>
        </div>
        <div class="bg-surface border-surface-muted rounded border p-4">
          <div class="text-fg-dim mb-1 text-xs tracking-wide uppercase">P95 Latency</div>
          <div class="text-fg text-2xl font-semibold">
            {latencyDistribution?.p95_latency ? formatDuration(latencyDistribution.p95_latency) : "N/A"}
          </div>
        </div>
      </div>

      {#if costStatsBySource.length > 0}
        <div class="bg-surface border-surface-muted overflow-hidden rounded border">
          <div class="border-surface-muted bg-surface-soft border-b px-4 py-3">
            <h3 class="text-fg m-0 text-sm font-semibold">Cost by Source</h3>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-surface-muted text-fg-dim border-b text-left text-xs">
                  <th class="px-4 py-2 font-medium">Source</th>
                  <th class="px-4 py-2 text-right font-medium">Sessions</th>
                  <th class="px-4 py-2 text-right font-medium">Total Cost</th>
                  <th class="px-4 py-2 text-right font-medium">Avg Cost/Session</th>
                  <th class="px-4 py-2 text-right font-medium">Input Tokens</th>
                  <th class="px-4 py-2 text-right font-medium">Output Tokens</th>
                </tr>
              </thead>
              <tbody>
                {#each costStatsBySource as stat (stat.dimension)}
                  <tr class="border-surface-muted hover:bg-surface-soft border-b last:border-b-0">
                    <td class="text-fg px-4 py-3 font-medium">{stat.dimension}</td>
                    <td class="px-4 py-3 text-right">{stat.session_count}</td>
                    <td class="px-4 py-3 text-right font-semibold">{formatCurrency(stat.total_cost)}</td>
                    <td class="text-fg-dim px-4 py-3 text-right">{formatCurrency(stat.avg_cost_per_session)}</td>
                    <td class="text-fg-dim px-4 py-3 text-right">
                      {stat.total_input_tokens ? (stat.total_input_tokens / 1000).toFixed(1) + "k" : "N/A"}
                    </td>
                    <td class="text-fg-dim px-4 py-3 text-right">
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
        <div class="bg-surface border-surface-muted overflow-hidden rounded border">
          <div class="border-surface-muted bg-surface-soft border-b px-4 py-3">
            <h3 class="text-fg m-0 text-sm font-semibold">Cost by Project</h3>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-surface-muted text-fg-dim border-b text-left text-xs">
                  <th class="px-4 py-2 font-medium">Project</th>
                  <th class="px-4 py-2 text-right font-medium">Sessions</th>
                  <th class="px-4 py-2 text-right font-medium">Total Cost</th>
                  <th class="px-4 py-2 text-right font-medium">Avg Cost/Session</th>
                </tr>
              </thead>
              <tbody>
                {#each costStatsByProject.slice(0, 10) as stat (stat.dimension)}
                  <tr class="border-surface-muted hover:bg-surface-soft border-b last:border-b-0">
                    <td class="text-fg px-4 py-3 font-medium">{stat.dimension}</td>
                    <td class="px-4 py-3 text-right">{stat.session_count}</td>
                    <td class="px-4 py-3 text-right font-semibold">{formatCurrency(stat.total_cost)}</td>
                    <td class="text-fg-dim px-4 py-3 text-right">{formatCurrency(stat.avg_cost_per_session)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
            {#if costStatsByProject.length > 10}
              <div class="text-fg-dim border-surface-muted border-t px-4 py-2 text-center text-xs">
                ... and {costStatsByProject.length - 10} more projects
              </div>
            {/if}
          </div>
        </div>
      {/if}

      {#if modelUsageStats.length > 0}
        <div class="bg-surface border-surface-muted overflow-hidden rounded border">
          <div class="border-surface-muted bg-surface-soft border-b px-4 py-3">
            <h3 class="text-fg m-0 text-sm font-semibold">Model Usage</h3>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-surface-muted text-fg-dim border-b text-left text-xs">
                  <th class="px-4 py-2 font-medium">Model/Provider</th>
                  <th class="px-4 py-2 text-right font-medium">Sessions</th>
                  <th class="px-4 py-2 text-right font-medium">Total Cost</th>
                  <th class="px-4 py-2 text-right font-medium">Input Tokens</th>
                  <th class="px-4 py-2 text-right font-medium">Output Tokens</th>
                  <th class="px-4 py-2 text-right font-medium">Avg Latency</th>
                </tr>
              </thead>
              <tbody>
                {#each modelUsageStats as stat (stat.model)}
                  <tr class="border-surface-muted hover:bg-surface-soft border-b last:border-b-0">
                    <td class="text-fg px-4 py-3 font-medium">{stat.provider}/{stat.model}</td>
                    <td class="px-4 py-3 text-right">{stat.session_count}</td>
                    <td class="px-4 py-3 text-right font-semibold">{formatCurrency(stat.total_cost)}</td>
                    <td class="text-fg-dim px-4 py-3 text-right">
                      {stat.total_input_tokens ? (stat.total_input_tokens / 1000).toFixed(1) + "k" : "N/A"}
                    </td>
                    <td class="text-fg-dim px-4 py-3 text-right">
                      {stat.total_output_tokens ? (stat.total_output_tokens / 1000).toFixed(1) + "k" : "N/A"}
                    </td>
                    <td class="text-fg-dim px-4 py-3 text-right">
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

<div class="bg-surface-soft flex h-full flex-col overflow-hidden">
  <div class="border-surface-muted bg-surface border-b p-4">
    <div class="mb-4 flex items-center justify-between">
      <h2 class="text-fg m-0 text-lg font-semibold">Analytics Dashboard</h2>
      <div class="flex gap-2">
        <select
          class="bg-surface-soft border-surface-muted text-fg font-inherit focus:border-blue cursor-pointer rounded border px-3 py-1.5 text-sm focus:outline-none"
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
          class="bg-blue text-surface font-inherit hover:bg-blue-bright cursor-pointer rounded border-none px-3 py-1.5 text-sm transition-colors disabled:opacity-50"
          onclick={loadAllStats}
          disabled={loading}>
          {loading ? "Loading..." : "Refresh"}
        </button>
      </div>
    </div>

    <div class="flex gap-1">
      {#each ["overview", "tools", "files", "churn", "latency", "efficiency"] as section (section)}
        <button
          class="text-fg-dim font-inherit hover:bg-surface-soft cursor-pointer rounded border-none bg-transparent px-4 py-2 text-sm transition-all"
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
    <div class="bg-red text-surface mx-4 my-2 rounded p-3 text-sm" transition:fade>{error}</div>
  {/if}

  <div class="flex-1 overflow-y-auto p-4">
    {#if loading && activityStats.length === 0}
      <div class="text-fg-dim flex h-full items-center justify-center">
        <div class="text-center">
          <div class="i-ri-loader-4-line mb-3 animate-spin text-3xl"></div>
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
