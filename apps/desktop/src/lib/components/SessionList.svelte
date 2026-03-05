<script lang="ts">
  import DataTable from "$lib/components/DataTable.svelte";
  import { bookmarkStore } from "$lib/stores/bookmarks.svelte";
  import type { DataTableColumn, DataTableRowAction, SessionData } from "$lib/types";
  import { SvelteSet } from "svelte/reactivity";
  import { fly } from "svelte/transition";

  type Props = {
    sessions: SessionData[];
    selectedSession: SessionData | null;
    onSelect: (session: SessionData) => void;
  };

  let { sessions, selectedSession, onSelect }: Props = $props();
  let sortBy = $state<"updated_at" | "created_at" | "title">("updated_at");
  let sortDirection = $state<"asc" | "desc">("desc");
  let pinnedFirst = $state(true);
  let pinnedOnly = $state(false);

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  }

  function getSessionTitle(session: SessionData): string {
    return session.title || session.external_id.slice(0, 8) || "Untitled";
  }

  function getProjectName(session: SessionData): string {
    if (!session.project) return "No project";
    const parts = session.project.split("-");
    return parts.at(-1) ?? session.project;
  }

  function getSourceBadgeClass(source: string): string {
    const classes: Record<string, string> = {
      claude: "bg-orange-bright text-white",
      codex: "bg-surface-hard text-fg-dim",
      opencode: "bg-surface-hard text-fg-dim",
      crush: "bg-purple-bright text-surface",
    };
    return classes[source.toLowerCase()] || "bg-surface-muted text-fg";
  }

  let pinnedSessionIds = $derived.by(() => {
    const ids = new SvelteSet<string>();
    for (const bookmark of bookmarkStore.bookmarks) {
      if (bookmark.type === "session" && bookmark.data.sessionId) {
        ids.add(bookmark.data.sessionId);
      }
    }
    return ids;
  });

  let sortedSessions = $derived.by(() => {
    const rows = pinnedOnly ? sessions.filter((session) => pinnedSessionIds.has(session.id)) : [...sessions];

    rows.sort((a, b) => {
      let aValue: string;
      let bValue: string;

      if (sortBy === "title") {
        aValue = getSessionTitle(a).toLowerCase();
        bValue = getSessionTitle(b).toLowerCase();
      } else {
        aValue = a[sortBy];
        bValue = b[sortBy];
      }

      const comparison = aValue.localeCompare(bValue);
      return sortDirection === "asc" ? comparison : -comparison;
    });

    if (pinnedFirst) {
      rows.sort((a, b) => Number(pinnedSessionIds.has(b.id)) - Number(pinnedSessionIds.has(a.id)));
    }

    return rows;
  });

  const columns: DataTableColumn<SessionData>[] = [
    { key: "title", header: "Title", filterable: true, render: (row) => getSessionTitle(row) },
    {
      key: "source",
      header: "Source",
      width: "100px",
      render: (row) => ({
        text: row.source,
        className: `text-2xs uppercase px-1.5 py-0.5 rounded shrink-0 ${getSourceBadgeClass(row.source)}`,
      }),
    },
    { key: "project", header: "Project", filterable: true, render: (row) => getProjectName(row) },
    { key: "updated_at", header: "Updated", width: "170px", render: (row) => formatDate(row.updated_at) },
  ];

  const rowActions: DataTableRowAction<SessionData>[] = [
    {
      id: "open",
      label: "Open",
      icon: "i-ri-external-link-line",
      title: "Open session",
      onClick: (row) => onSelect(row),
    },
  ];
</script>

<div class="flex h-full flex-col overflow-hidden" in:fly={{ y: 10, duration: 200 }}>
  <div class="border-surface-muted bg-surface-soft space-y-2 border-b px-4 py-3">
    <div class="flex items-center justify-between">
      <span class="text-fg-muted text-xs">{sortedSessions.length} sessions</span>
      <span class="text-fg-dim text-xs">{pinnedSessionIds.size} pinned</span>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <label class="text-fg-dim flex items-center gap-1 text-xs">
        <span>Sort</span>
        <select class="bg-surface border-surface-muted text-fg rounded border px-2 py-1 text-xs" bind:value={sortBy}>
          <option value="updated_at">Updated</option>
          <option value="created_at">Created</option>
          <option value="title">Title</option>
        </select>
      </label>
      <button
        class="bg-surface border-surface-muted text-fg-dim hover:text-fg rounded border px-2 py-1 text-xs"
        onclick={() => (sortDirection = sortDirection === "asc" ? "desc" : "asc")}
        type="button"
        title="Toggle sort direction">
        {sortDirection === "asc" ? "Asc" : "Desc"}
      </button>
      <button
        class="rounded border px-2 py-1 text-xs transition-colors {pinnedFirst
          ? 'bg-blue/15 border-blue text-blue'
          : 'bg-surface border-surface-muted text-fg-dim hover:text-fg'}"
        onclick={() => (pinnedFirst = !pinnedFirst)}
        type="button">
        Pinned first
      </button>
      <button
        class="rounded border px-2 py-1 text-xs transition-colors {pinnedOnly
          ? 'bg-blue/15 border-blue text-blue'
          : 'bg-surface border-surface-muted text-fg-dim hover:text-fg'}"
        onclick={() => (pinnedOnly = !pinnedOnly)}
        type="button">
        Pinned only
      </button>
    </div>
  </div>

  <DataTable
    data={sortedSessions}
    {columns}
    {rowActions}
    expandableRows
    keyExtractor={(row) => row.id}
    pageSize={100}
    {onSelect}
    selectedId={selectedSession?.id ?? null} />
</div>
