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

<div class="app-container">
  <aside class="sidebar">
    <div class="sidebar-header">
      <h1 class="app-title">Agent Viz</h1>
      <button class="btn-primary" onclick={ingestClaude} disabled={loading}>
        {loading ? "Loading..." : "Ingest Claude"}
      </button>
    </div>

    <div class="tab-bar">
      <button class="tab-button" class:active={activeTab === "sessions"} onclick={() => (activeTab = "sessions")}>
        Sessions
      </button>
      <button class="tab-button" class:active={activeTab === "search"} onclick={() => (activeTab = "search")}>
        Search
      </button>
    </div>

    {#if error}
      <div class="error-message">{error}</div>
    {/if}

    {#if activeTab === "sessions"}
      <SessionList {sessions} {selectedSession} onSelect={selectSession} />
    {:else}
      <SearchPanel onSelectSession={selectSessionById} />
    {/if}
  </aside>

  <main class="main-content">
    {#if selectedSession}
      <SessionViewer session={selectedSession} {events} />
    {:else}
      <div class="empty-state">
        <p>Select a session to view details</p>
      </div>
    {/if}
  </main>
</div>

<style>
  .app-container {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .sidebar {
    width: 320px;
    min-width: 320px;
    background-color: var(--color-bg-soft);
    border-right: 1px solid var(--color-bg-muted);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    padding: 1rem;
    border-bottom: 1px solid var(--color-bg-muted);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .app-title {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--color-fg);
  }

  .btn-primary {
    padding: 0.5rem 1rem;
    background-color: var(--color-blue);
    color: var(--color-bg);
    border: none;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.875rem;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .btn-primary:hover:not(:disabled) {
    background-color: var(--color-blue-bright);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error-message {
    margin: 0.5rem 1rem;
    padding: 0.5rem;
    background-color: var(--color-red);
    color: var(--color-bg);
    border-radius: 4px;
    font-size: 0.75rem;
  }

  .main-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-fg-dim);
  }

  .tab-bar {
    display: flex;
    border-bottom: 1px solid var(--color-bg-muted);
    background-color: var(--color-bg);
  }

  .tab-button {
    flex: 1;
    padding: 0.75rem;
    background-color: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--color-fg-dim);
    font-family: inherit;
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .tab-button:hover {
    color: var(--color-fg);
    background-color: var(--color-bg-soft);
  }

  .tab-button.active {
    color: var(--color-blue);
    border-bottom-color: var(--color-blue);
    background-color: var(--color-bg-soft);
  }
</style>
