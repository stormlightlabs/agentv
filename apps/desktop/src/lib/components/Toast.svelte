<script lang="ts">
  import type { ToastNotification } from "$lib/types";
  import { fly } from "svelte/transition";

  type Props = { notification: ToastNotification; onDismiss: (id: string) => void };

  let { notification, onDismiss }: Props = $props();

  $effect(() => {
    const duration = notification.duration || 5000;
    const timer = setTimeout(() => {
      onDismiss(notification.id);
    }, duration);

    return () => clearTimeout(timer);
  });

  function getIcon(): string {
    switch (notification.type) {
      case "success": {
        return "i-ri-checkbox-circle-line";
      }
      case "error": {
        return "i-ri-close-circle-line";
      }
      default: {
        return "i-ri-information-line";
      }
    }
  }

  function getColors(): string {
    switch (notification.type) {
      case "success": {
        return "border-green bg-surface-soft";
      }
      case "error": {
        return "border-red bg-surface-soft";
      }
      default: {
        return "border-blue bg-surface-soft";
      }
    }
  }

  function getIconColor(): string {
    switch (notification.type) {
      case "success": {
        return "text-green";
      }
      case "error": {
        return "text-red";
      }
      default: {
        return "text-blue";
      }
    }
  }
</script>

<div
  class="flex max-w-[400px] min-w-[300px] items-start gap-3 rounded-lg border p-4 shadow-lg {getColors()}"
  in:fly={{ x: 100, duration: 300 }}
  out:fly={{ x: 100, duration: 200 }}
  role="alert">
  <span class="{getIcon()} {getIconColor()} mt-0.5 shrink-0 text-xl"></span>
  <div class="flex-1">
    <p class="text-fg m-0 text-sm">{notification.message}</p>
  </div>
  <button
    class="text-fg-dim hover:text-fg cursor-pointer border-none bg-transparent p-0 transition-colors"
    onclick={() => onDismiss(notification.id)}
    aria-label="Dismiss notification">
    <span class="i-ri-close-line"></span>
  </button>
</div>
