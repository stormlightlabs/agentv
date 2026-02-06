<script lang="ts">
  import { cubicOut } from "svelte/easing";
  import { fade, fly, scale, slide } from "svelte/transition";

  type Props = {
    children: import("svelte").Snippet;
    type?: "fade" | "fly" | "slide" | "scale";
    duration?: number;
    delay?: number;
    x?: number;
    y?: number;
  };

  let { children, type = "fade", duration = 200, delay = 0, x = 0, y = 10 }: Props = $props();

  const easing = cubicOut;

  function getTransition(node: Element) {
    const params = { duration, delay, easing };
    switch (type) {
      case "fade":
        return fade(node, params);
      case "fly":
        return fly(node, { ...params, x, y });
      case "slide":
        return slide(node, params);
      case "scale":
        return scale(node, { ...params, start: 0.95 });
      default:
        return fade(node, params);
    }
  }
</script>

<div transition:getTransition>
  {@render children()}
</div>
