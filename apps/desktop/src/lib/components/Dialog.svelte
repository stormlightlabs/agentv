<script lang="ts">
  import type { Snippet } from "svelte";
  import { fade } from "svelte/transition";

  type Props = {
    open: boolean;
    onOpenChange?: (open: boolean) => void;
    children?: Snippet;
    class?: string;
    closeOnEscape?: boolean;
    closeOnOutsideClick?: boolean;
    role?: "dialog" | "alertdialog";
    "aria-label"?: string;
    "aria-labelledby"?: string;
    "aria-describedby"?: string;
  };

  let {
    open = $bindable(),
    onOpenChange,
    children,
    class: className = "",
    closeOnEscape = true,
    closeOnOutsideClick = true,
    role = "dialog",
    "aria-label": ariaLabel,
    "aria-labelledby": ariaLabelledBy,
    "aria-describedby": ariaDescribedBy,
  }: Props = $props();

  let containerRef: HTMLDivElement | null = $state(null);

  function handleClose() {
    open = false;
    onOpenChange?.(false);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (closeOnEscape && event.key === "Escape") {
      event.preventDefault();
      handleClose();
    }
  }

  function handleBackdropClick(event: MouseEvent) {
    if (closeOnOutsideClick && event.target === containerRef) {
      handleClose();
    }
  }

  $effect(() => {
    if (open && containerRef) {
      const focusable = containerRef.querySelector<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      focusable?.focus();
    }
  });
</script>

{#if open}
  <div
    bind:this={containerRef}
    class="fixed inset-0 z-50 {className}"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    {role}
    aria-modal="true"
    aria-label={ariaLabel}
    aria-labelledby={ariaLabelledBy}
    aria-describedby={ariaDescribedBy}
    tabindex="-1"
    transition:fade={{ duration: 150 }}>
    {@render children?.()}
  </div>
{/if}
