<script lang="ts">
  import DataTable from "$lib/components/DataTable.svelte";
  import type { DataTableColumn, SessionData } from "$lib/types";
  import { fly } from "svelte/transition";

  type Props = {
    sessions: SessionData[];
    selectedSession: SessionData | null;
    onSelect: (session: SessionData) => void;
  };

  let { sessions, selectedSession, onSelect }: Props = $props();

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
    return parts[parts.length - 1] || session.project;
  }

  function getSourceBadgeClass(source: string): string {
    const classes: Record<string, string> = {
      claude: "bg-blue text-surface",
      codex: "bg-green text-surface",
      opencode: "bg-purple text-surface",
      crush: "bg-yellow text-surface",
    };
    return classes[source.toLowerCase()] || "bg-surface-muted text-fg";
  }

  const columns: DataTableColumn<SessionData>[] = [
    { key: "title", header: "Title", sortable: true, filterable: true, render: (row) => getSessionTitle(row) },
    {
      key: "source",
      header: "Source",
      sortable: true,
      filterable: true,
      width: "100px",
      render: (row) => ({
        text: row.source,
        className: `text-2xs uppercase px-1.5 py-0.5 rounded shrink-0 ${getSourceBadgeClass(row.source)}`,
      }),
    },
    { key: "project", header: "Project", sortable: true, filterable: true, render: (row) => getProjectName(row) },
    {
      key: "updated_at",
      header: "Updated",
      sortable: true,
      width: "120px",
      render: (row) => formatDate(row.updated_at),
    },
  ];
</script>

<div class="flex flex-col h-full overflow-hidden" in:fly={{ y: 10, duration: 200 }}>
  <div class="px-4 py-2 border-b border-surface-muted flex justify-between items-center bg-surface-soft">
    <span class="text-xs text-fg-muted">{sessions.length} sessions</span>
  </div>

  <DataTable
    data={sessions}
    {columns}
    keyExtractor={(row) => row.id}
    pageSize={100}
    {onSelect}
    selectedId={selectedSession?.id ?? null} />
</div>
