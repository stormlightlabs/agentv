<script lang="ts">
  import Drawer from "$lib/components/Drawer.svelte";
  import SessionList from "$lib/components/SessionList.svelte";
  import type { SessionData } from "$lib/types";

  type Props = {
    open: boolean;
    filteredSessions: SessionData[];
    selectedSession: SessionData | null;
    onOpenChange?: (open: boolean) => void;
    onSelectSession: (session: SessionData) => void;
  };

  let { open, filteredSessions, selectedSession, onOpenChange, onSelectSession }: Props = $props();
</script>

<Drawer {open} {onOpenChange} direction="left" size="lg" aria-label="Session list drawer">
  <div class="bg-surface-soft flex h-full flex-col">
    <div class="border-surface-muted flex items-center justify-between border-b px-4 py-3">
      <h2 class="text-fg m-0 text-sm font-semibold">Sessions</h2>
      <span class="text-2xs text-fg-dim">{filteredSessions.length}</span>
    </div>
    <div class="flex-1 overflow-hidden">
      <SessionList sessions={filteredSessions} {selectedSession} onSelect={onSelectSession} />
    </div>
  </div>
</Drawer>
