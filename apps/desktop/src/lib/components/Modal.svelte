<script lang="ts">
  import type { Snippet } from "svelte";
  import { scale } from "svelte/transition";
  import Dialog from "./Dialog.svelte";

  type Props = {
    open: boolean;
    onOpenChange?: (open: boolean) => void;
    children?: Snippet;
    class?: string;
    contentClass?: string;
    closeOnEscape?: boolean;
    closeOnOutsideClick?: boolean;
    size?: "sm" | "md" | "lg" | "xl" | "full";
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
    contentClass = "",
    closeOnEscape = true,
    closeOnOutsideClick = true,
    size = "md",
    role = "dialog",
    "aria-label": ariaLabel,
    "aria-labelledby": ariaLabelledBy,
    "aria-describedby": ariaDescribedBy,
  }: Props = $props();

  const sizeClasses = {
    sm: "max-w-md",
    md: "max-w-2xl",
    lg: "max-w-4xl",
    xl: "max-w-6xl",
    full: "max-w-[calc(100vw-2rem)]",
  };
</script>

<Dialog
  bind:open
  {onOpenChange}
  class="bg-black/50 flex items-center justify-center p-4 {className}"
  {closeOnEscape}
  {closeOnOutsideClick}
  {role}
  aria-label={ariaLabel}
  aria-labelledby={ariaLabelledBy}
  aria-describedby={ariaDescribedBy}>
  <div
    class="w-full {sizeClasses[size]} bg-surface rounded-lg shadow-2xl overflow-hidden {contentClass}"
    transition:scale={{ start: 0.95, duration: 150 }}>
    {@render children?.()}
  </div>
</Dialog>
