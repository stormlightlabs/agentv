<script lang="ts">
  import { fade } from "svelte/transition";

  type Props = {
    onOpenCommandPalette?: () => void;
    onFollowLatestSession?: () => void;
  };

  let { onOpenCommandPalette, onFollowLatestSession }: Props = $props();
</script>

<div class="text-fg-dim flex flex-1 items-center justify-center px-6" in:fade>
  <div class="max-w-lg text-center">
    <div class="i-ri-chat-3-line mb-3 text-4xl opacity-50"></div>
    <p class="text-fg m-0 text-base">Select a session</p>
    <p class="text-fg-muted mt-2 mb-4 text-sm">
      Choose a session from the left pane to inspect timeline details.
    </p>
    <div class="text-fg-dim space-y-1 text-sm">
      <p class="m-0">Press Cmd+K to jump to a command or session.</p>
      <p class="m-0">Use the top search and filter chips to narrow results.</p>
      <p class="m-0">Use Live after opening a session to follow new events.</p>
    </div>
    <div class="mt-4 flex flex-wrap items-center justify-center gap-2">
      <button
        class="bg-surface-soft border-surface-muted text-fg-dim hover:text-fg rounded border px-3 py-2 text-xs"
        onclick={onOpenCommandPalette}
        type="button">
        Cmd+K Command Palette
      </button>
      <button
        class="bg-surface-soft border-surface-muted text-fg-dim hover:text-fg rounded border px-3 py-2 text-xs"
        onclick={onFollowLatestSession}
        type="button">
        Open Latest Session
      </button>
    </div>
  </div>
</div>
