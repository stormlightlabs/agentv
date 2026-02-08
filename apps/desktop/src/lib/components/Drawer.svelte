<script lang="ts">
  import type { Snippet } from "svelte";
  import { slide } from "svelte/transition";
  import Dialog from "./Dialog.svelte";

  type Props = {
    open: boolean;
    onOpenChange?: (open: boolean) => void;
    children?: Snippet;
    class?: string;
    contentClass?: string;
    closeOnEscape?: boolean;
    closeOnOutsideClick?: boolean;
    direction?: "top" | "right" | "bottom" | "left";
    size?: "sm" | "md" | "lg" | "xl";
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
    direction = "right",
    size = "md",
    role = "dialog",
    "aria-label": ariaLabel,
    "aria-labelledby": ariaLabelledBy,
    "aria-describedby": ariaDescribedBy,
  }: Props = $props();

  const sizeClasses = $derived({
    sm: direction === "top" || direction === "bottom" ? "h-1/4" : "w-72",
    md: direction === "top" || direction === "bottom" ? "h-1/3" : "w-80",
    lg: direction === "top" || direction === "bottom" ? "h-1/2" : "w-96",
    xl: direction === "top" || direction === "bottom" ? "h-2/3" : "w-[28rem]",
  });

  const directionClasses = $derived({
    top: "top-0 left-0 right-0 " + sizeClasses[size] + " border-b",
    right: "top-0 right-0 bottom-0 " + sizeClasses[size] + " border-l",
    bottom: "bottom-0 left-0 right-0 " + sizeClasses[size] + " border-t",
    left: "top-0 left-0 bottom-0 " + sizeClasses[size] + " border-r",
  });

  const positionClasses = {
    top: "items-start justify-center",
    right: "items-center justify-end",
    bottom: "items-end justify-center",
    left: "items-center justify-start",
  };

  const isHorizontal = $derived(direction === "left" || direction === "right");
  const transitionParams = $derived(
    isHorizontal ? { axis: "x" as const, duration: 200 } : { axis: "y" as const, duration: 200 },
  );
</script>

<Dialog
  bind:open
  {onOpenChange}
  class="bg-black/50 flex {positionClasses[direction]} {className}"
  {closeOnEscape}
  {closeOnOutsideClick}
  {role}
  aria-label={ariaLabel}
  aria-labelledby={ariaLabelledBy}
  aria-describedby={ariaDescribedBy}>
  <div
    class="absolute {directionClasses[
      direction
    ]} bg-surface border-surface-muted shadow-2xl flex flex-col overflow-hidden {contentClass}"
    transition:slide={transitionParams}>
    {@render children?.()}
  </div>
</Dialog>
