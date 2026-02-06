<script lang="ts">
  import DataTable from "$lib/components/DataTable.svelte";
  import SearchPanel from "$lib/components/SearchPanel.svelte";
  import SessionList from "$lib/components/SessionList.svelte";
  import SessionViewer from "$lib/components/SessionViewer.svelte";
  import StatusPanel from "$lib/components/StatusPanel.svelte";
  import Toast from "$lib/components/Toast.svelte";
  import WelcomeScreen from "$lib/components/WelcomeScreen.svelte";
  import { useToast } from "$lib/stores/toast";
  import type { EventData, IngestResult, SessionData } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";

  const toast = useToast();

  let sessions = $state<SessionData[]>([]);
  let selectedSession = $state<SessionData | null>(null);
  let events = $state<EventData[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let activeTab = $state<"sessions" | "search" | "status">("sessions");
  let ingestLoading = $state(false);
  let lastIngestTime = $state<Date | null>(null);

  const sources = [
    { id: "claude", name: "Claude", color: "blue" },
    { id: "codex", name: "Codex", color: "green" },
    { id: "opencode", name: "OpenCode", color: "purple" },
    { id: "crush", name: "Crush", color: "yellow" },
  ];

  async function loadSessions() {
    try {
      loading = true;
      error = null;
      sessions = await invoke<SessionData[]>("list_sessions");
    } catch (e) {
      error = String(e);
      toast.error(`Failed to load sessions: ${e}`);
    } finally {
      loading = false;
    }
  }

  async function selectSession(session: SessionData) {
    selectedSession = session;
    try {
      events = await invoke<EventData[]>("get_session_events", { sessionId: session.id });
    } catch (e) {
      console.error("Failed to load events:", e);
      toast.error(`Failed to load events: ${e}`);
      events = [];
    }
  }

  async function selectSessionById(sessionId: string) {
    const session = sessions.find((s) => s.id === sessionId);
    if (session) {
      await selectSession(session);
      activeTab = "sessions";
    }
  }

  async function ingestSource(sourceId: string) {
    ingestLoading = true;
    error = null;

    try {
      const result = await invoke<IngestResult>("ingest_source", { source: sourceId });
      lastIngestTime = new Date();

      if (result.imported > 0) {
        toast.success(`Imported ${result.imported} sessions from ${sourceId} in ${result.duration_ms}ms`);
      } else if (result.failed > 0) {
        toast.error(`Failed to import ${result.failed} sessions from ${sourceId}`);
      } else {
        toast.info(`No new sessions found in ${sourceId}`);
      }

      await loadSessions();
    } catch (e) {
      error = String(e);
      toast.error(`Failed to ingest ${sourceId}: ${e}`);
    } finally {
      ingestLoading = false;
    }
  }

  async function ingestAllSources() {
    ingestLoading = true;
    error = null;

    try {
      const results = await invoke<IngestResult[]>("ingest_all_sources");
      lastIngestTime = new Date();

      const totalImported = results.reduce((sum, r) => sum + r.imported, 0);
      const totalFailed = results.reduce((sum, r) => sum + r.failed, 0);

      if (totalImported > 0) {
        toast.success(`Imported ${totalImported} sessions from all sources`);
      } else if (totalFailed > 0) {
        toast.error(`Failed to import ${totalFailed} sessions`);
      } else {
        toast.info("No new sessions found");
      }

      await loadSessions();
    } catch (e) {
      error = String(e);
      toast.error(`Failed to ingest all sources: ${e}`);
    } finally {
      ingestLoading = false;
    }
  }

  onMount(() => {
    loadSessions();
  });
</script>

<!-- Toast Notifications -->
<div class="fixed top-4 right-4 z-50 flex flex-col gap-2">
  {#each toast.notifications as notification (notification.id)}
    <Toast {notification} onDismiss={toast.removeToast} />
  {/each}
</div>

<div class="flex h-screen overflow-hidden">
  <aside class="w-80 min-w-80 bg-bg-soft border-r border-bg-muted flex flex-col overflow-hidden">
    <div class="p-4 border-b border-bg-muted flex flex-col gap-3">
      <h1 class="m-0 text-xl font-semibold text-fg">Agent Viz</h1>

      <button
        class="px-4 py-2 bg-blue text-bg border-none rounded font-inherit text-sm cursor-pointer transition-colors hover:not-disabled:bg-blue-bright disabled:opacity-50 disabled:cursor-not-allowed"
        onclick={ingestAllSources}
        disabled={ingestLoading}>
        {#if ingestLoading}
          <span class="i-ri-loader-4-line animate-spin mr-1"></span>
          Ingesting...
        {:else}
          <span class="i-ri-refresh-line mr-1"></span>
          Refresh All
        {/if}
      </button>

      <div class="flex flex-wrap gap-1">
        {#each sources as source}
          <button
            class="px-2 py-1 bg-bg border border-bg-muted rounded text-2xs text-fg cursor-pointer transition-all hover:border-{source.color} hover:text-{source.color} disabled:opacity-50"
            onclick={() => ingestSource(source.id)}
            disabled={ingestLoading}
            title="Refresh {source.name}">
            {source.name}
          </button>
        {/each}
      </div>
    </div>

    <div class="flex border-b border-bg-muted bg-bg">
      <button
        class="flex-1 px-3 py-3 bg-transparent border-none border-b-2 border-transparent text-fg-dim font-inherit text-sm cursor-pointer transition-all hover:text-fg hover:bg-bg-soft"
        class:active={activeTab === "sessions"}
        class:text-blue={activeTab === "sessions"}
        class:border-b-blue={activeTab === "sessions"}
        class:bg-bg-soft={activeTab === "sessions"}
        onclick={() => (activeTab = "sessions")}>
        Sessions
      </button>
      <button
        class="flex-1 px-3 py-3 bg-transparent border-none border-b-2 border-transparent text-fg-dim font-inherit text-sm cursor-pointer transition-all hover:text-fg hover:bg-bg-soft"
        class:active={activeTab === "search"}
        class:text-blue={activeTab === "search"}
        class:border-b-blue={activeTab === "search"}
        class:bg-bg-soft={activeTab === "search"}
        onclick={() => (activeTab = "search")}>
        Search
      </button>
      <button
        class="flex-1 px-3 py-3 bg-transparent border-none border-b-2 border-transparent text-fg-dim font-inherit text-sm cursor-pointer transition-all hover:text-fg hover:bg-bg-soft"
        class:active={activeTab === "status"}
        class:text-blue={activeTab === "status"}
        class:border-b-blue={activeTab === "status"}
        class:bg-bg-soft={activeTab === "status"}
        onclick={() => (activeTab = "status")}>
        Status
      </button>
    </div>

    {#if error}
      <div class="mx-4 my-2 p-2 bg-red text-bg rounded text-xs" transition:fade>
        {error}
      </div>
    {/if}

    <div class="flex-1 overflow-hidden">
      {#if activeTab === "sessions"}
        <SessionList {sessions} {selectedSession} onSelect={selectSession} />
      {:else if activeTab === "search"}
        <SearchPanel onSelectSession={selectSessionById} />
      {:else}
        <StatusPanel onRefresh={loadSessions} />
      {/if}
    </div>

    <div class="p-2 border-t border-bg-muted bg-bg text-xs text-fg-dim">
      <div class="flex justify-between items-center">
        <span>{sessions.length} sessions</span>
        {#if lastIngestTime}
          <span>
            Last update: {lastIngestTime.toLocaleTimeString()}
          </span>
        {/if}
      </div>
    </div>
  </aside>

  <main class="flex-1 overflow-hidden flex flex-col">
    {#if sessions.length === 0 && !loading}
      <WelcomeScreen onGetStarted={ingestAllSources} />
    {:else if selectedSession}
      <SessionViewer session={selectedSession} {events} />
    {:else}
      <div class="flex-1 flex items-center justify-center text-fg-dim" in:fade>
        <div class="text-center">
          <div class="i-ri-chat-3-line text-4xl mb-3 opacity-50"></div>
          <p>Select a session to view details</p>
        </div>
      </div>
    {/if}
  </main>
</div>
