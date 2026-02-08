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
      case "success":
        return "i-ri-checkbox-circle-line";
      case "error":
        return "i-ri-close-circle-line";
      case "info":
      default:
        return "i-ri-information-line";
    }
  }

  function getColors(): string {
    switch (notification.type) {
      case "success":
        return "border-green bg-surface-soft";
      case "error":
        return "border-red bg-surface-soft";
      case "info":
      default:
        return "border-blue bg-surface-soft";
    }
  }

  function getIconColor(): string {
    switch (notification.type) {
      case "success":
        return "text-green";
      case "error":
        return "text-red";
      case "info":
      default:
        return "text-blue";
    }
  }
</script>

<div
  class="flex items-start gap-3 p-4 rounded-lg border shadow-lg min-w-[300px] max-w-[400px] {getColors()}"
  in:fly={{ x: 100, duration: 300 }}
  out:fly={{ x: 100, duration: 200 }}
  role="alert">
  <span class="{getIcon()} {getIconColor()} text-xl shrink-0 mt-0.5"></span>
  <div class="flex-1">
    <p class="text-sm text-fg m-0">{notification.message}</p>
  </div>
  <button
    class="bg-transparent border-none p-0 text-fg-dim hover:text-fg cursor-pointer transition-colors"
    onclick={() => onDismiss(notification.id)}
    aria-label="Dismiss notification">
    <span class="i-ri-close-line"></span>
  </button>
</div>
