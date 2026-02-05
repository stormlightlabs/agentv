<script lang="ts">
  import SearchPanel from "$lib/components/SearchPanel.svelte";
  import SessionList from "$lib/components/SessionList.svelte";
  import SessionViewer from "$lib/components/SessionViewer.svelte";
  import type { EventData, SessionData } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let sessions = $state<SessionData[]>([]);
  let selectedSession = $state<SessionData | null>(null);
  let events = $state<EventData[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let activeTab = $state<"sessions" | "search">("sessions");

  async function loadSessions() {
    try {
      loading = true;
      error = null;
      sessions = await invoke("list_sessions");
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function selectSession(session: SessionData) {
    selectedSession = session;
    try {
      events = await invoke("get_session_events", { sessionId: session.id });
    } catch (e) {
      console.error("Failed to load events:", e);
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

  async function ingestClaude() {
    try {
      loading = true;
      error = null;
      const result: { imported: number; failed: number; total: number } = await invoke("ingest_source", {
        source: "claude",
      });
      await loadSessions();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadSessions();
  });
</script>

<div class="flex h-screen overflow-hidden">
  <aside class="w-80 min-w-80 bg-bg-soft border-r border-bg-muted flex flex-col overflow-hidden">
    <div class="p-4 border-b border-bg-muted flex flex-col gap-3">
      <h1 class="m-0 text-xl font-semibold text-fg">Agent Viz</h1>
      <button
        class="px-4 py-2 bg-blue text-bg border-none rounded font-inherit text-sm cursor-pointer transition-colors hover:not-disabled:bg-blue-bright disabled:opacity-50 disabled:cursor-not-allowed"
        onclick={ingestClaude}
        disabled={loading}>
        {loading ? "Loading..." : "Ingest Claude"}
      </button>
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
    </div>

    {#if error}
      <div class="mx-4 my-2 p-2 bg-red text-bg rounded text-xs">{error}</div>
    {/if}

    {#if activeTab === "sessions"}
      <SessionList {sessions} {selectedSession} onSelect={selectSession} />
    {:else}
      <SearchPanel onSelectSession={selectSessionById} />
    {/if}
  </aside>

  <main class="flex-1 overflow-hidden flex flex-col">
    {#if selectedSession}
      <SessionViewer session={selectedSession} {events} />
    {:else}
      <div class="flex-1 flex items-center justify-center text-fg-dim">
        <p>Select a session to view details</p>
      </div>
    {/if}
  </main>
</div>
