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
    side?: "left" | "right";
    width?: "sm" | "md" | "lg" | "xl" | "full";
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
    side = "right",
    width = "md",
    role = "dialog",
    "aria-label": ariaLabel,
    "aria-labelledby": ariaLabelledBy,
    "aria-describedby": ariaDescribedBy,
  }: Props = $props();

  const widthClasses = { sm: "w-72", md: "w-80", lg: "w-96", xl: "w-[28rem]", full: "w-full max-w-md" };

  const sideClasses = { left: "left-0 border-r", right: "right-0 border-l" };

  const slideAxis = { left: "x" as const, right: "x" as const };
</script>

<Dialog
  bind:open
  {onOpenChange}
  class={className}
  {closeOnEscape}
  {closeOnOutsideClick}
  {role}
  aria-label={ariaLabel}
  aria-labelledby={ariaLabelledBy}
  aria-describedby={ariaDescribedBy}>
  <div
    class="absolute top-0 {sideClasses[side]} h-full {widthClasses[
      width
    ]} bg-surface border-surface-muted shadow-xl flex flex-col {contentClass}"
    transition:slide={{ axis: slideAxis[side], duration: 200 }}>
    {@render children?.()}
  </div>
</Dialog>
