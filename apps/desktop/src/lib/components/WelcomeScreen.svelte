<script lang="ts">
  import { cubicOut } from "svelte/easing";
  import { fade, fly, scale } from "svelte/transition";

  type Props = { onGetStarted: () => void };

  let { onGetStarted }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center h-full p-8" in:fade={{ duration: 300 }}>
  <div class="text-center max-w-md">
    <div
      class="w-24 h-24 mx-auto mb-6 rounded-2xl bg-linear-to-br from-blue to-purple flex items-center justify-center shadow-lg"
      in:scale={{ duration: 400, delay: 100, start: 0.5, easing: cubicOut }}>
      <span class="i-ri-brain-line text-5xl text-surface"></span>
    </div>

    <h1 class="text-3xl font-bold text-fg mb-3" in:fly={{ y: 20, duration: 300, delay: 200 }}>Agent V</h1>

    <p class="text-fg-dim text-lg mb-8" in:fly={{ y: 20, duration: 300, delay: 300 }}>
      Visualize your AI agent sessions in one place
    </p>

    <div class="grid grid-cols-2 gap-4 mb-8 text-left" in:fly={{ y: 20, duration: 300, delay: 400 }}>
      <div class="flex items-start gap-2 p-3 bg-surface-soft rounded-lg border border-surface-muted">
        <span class="i-ri-robot-2-line text-blue mt-0.5"></span>
        <div>
          <div class="font-medium text-fg text-sm">Multi-Source</div>
          <div class="text-xs text-fg-dim">Claude, Codex, OpenCode & Crush</div>
        </div>
      </div>

      <div class="flex items-start gap-2 p-3 bg-surface-soft rounded-lg border border-surface-muted">
        <span class="i-ri-search-line text-purple mt-0.5"></span>
        <div>
          <div class="font-medium text-fg text-sm">Full-Text Search</div>
          <div class="text-xs text-fg-dim">Find anything instantly</div>
        </div>
      </div>

      <div class="flex items-start gap-2 p-3 bg-surface-soft rounded-lg border border-surface-muted">
        <span class="i-ri-bar-chart-box-line text-green mt-0.5"></span>
        <div>
          <div class="font-medium text-fg text-sm">Analytics</div>
          <div class="text-xs text-fg-dim">Track activity & errors</div>
        </div>
      </div>

      <div class="flex items-start gap-2 p-3 bg-surface-soft rounded-lg border border-surface-muted">
        <span class="i-ri-timeline-view text-yellow mt-0.5"></span>
        <div>
          <div class="font-medium text-fg text-sm">Timeline View</div>
          <div class="text-xs text-fg-dim">Browse session history</div>
        </div>
      </div>
    </div>

    <button
      class="px-8 py-3 bg-blue text-surface border-none rounded-lg font-semibold text-base cursor-pointer transition-all hover:bg-blue-bright hover:scale-105 active:scale-95 shadow-lg"
      onclick={onGetStarted}
      in:fly={{ y: 20, duration: 300, delay: 500 }}>
      Get Started
    </button>

    <p class="mt-4 text-xs text-fg-dim" in:fly={{ y: 20, duration: 300, delay: 600 }}>
      Click to import sessions from your configured sources
    </p>
  </div>
</div>
