<script lang="ts">
  import { keyboardStore, type CommandPaletteItem } from "$lib/stores/keyboard.svelte";
  import { onMount } from "svelte";
  import Modal from "./Modal.svelte";

  let selectedIndex = $state(0);
  let inputRef: HTMLInputElement | null = $state(null);

  $effect(() => {
    if (keyboardStore.commandPaletteOpen) {
      selectedIndex = 0;
      inputRef?.focus();
    }
  });

  $effect(() => {
    selectedIndex = 0;
  });

  function handleKeydown(event: KeyboardEvent) {
    const items = keyboardStore.filteredCommands;

    if (event.key === "Escape") {
      keyboardStore.closeCommandPalette();
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex = (selectedIndex + 1) % items.length;
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex = selectedIndex <= 0 ? items.length - 1 : selectedIndex - 1;
      return;
    }

    if (event.key === "Enter" && items.length > 0) {
      event.preventDefault();
      items[selectedIndex]?.action();
      keyboardStore.closeCommandPalette();
      return;
    }

    if ((event.metaKey || event.ctrlKey) && event.key === "k") {
      event.preventDefault();
      keyboardStore.openCommandPalette();
    }
  }

  function handleSelect(item: CommandPaletteItem) {
    item.action();
    keyboardStore.closeCommandPalette();
  }

  function getCategoryIcon(category: string): string {
    switch (category) {
      case "navigation":
        return "i-ri-compass-3-line";
      case "action":
        return "i-ri-flashlight-line";
      case "search":
        return "i-ri-search-line";
      case "export":
        return "i-ri-download-line";
      case "view":
        return "i-ri-eye-line";
      default:
        return "i-ri-circle-fill";
    }
  }

  function getCategoryColor(category: string): string {
    switch (category) {
      case "navigation":
        return "text-blue";
      case "action":
        return "text-yellow";
      case "search":
        return "text-green";
      case "export":
        return "text-purple";
      case "view":
        return "text-cyan";
      default:
        return "text-fg-muted";
    }
  }

  onMount(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        keyboardStore.openCommandPalette();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });
</script>

<Modal
  bind:open={keyboardStore.commandPaletteOpen}
  size="md"
  class="!items-start !pt-[20vh]"
  contentClass="border border-bg-muted"
  aria-label="Command palette">
  <div class="flex items-center gap-3 px-4 py-3 border-b border-bg-muted">
    <span class="i-ri-command-line text-fg-muted"></span>
    <input
      bind:this={inputRef}
      type="text"
      class="flex-1 bg-transparent border-none text-fg font-inherit text-base focus:outline-none placeholder-fg-muted"
      placeholder="Type a command or search..."
      bind:value={keyboardStore.commandPaletteSearch}
      onkeydown={handleKeydown} />
    <div class="flex gap-1">
      <kbd class="px-2 py-1 bg-bg-muted rounded text-xs text-fg-dim">↑↓</kbd>
      <kbd class="px-2 py-1 bg-bg-muted rounded text-xs text-fg-dim">↵</kbd>
      <kbd class="px-2 py-1 bg-bg-muted rounded text-xs text-fg-dim">Esc</kbd>
    </div>
  </div>

  <div class="max-h-[50vh] overflow-y-auto">
    {#if keyboardStore.filteredCommands.length === 0}
      <div class="px-4 py-8 text-center text-fg-dim">
        <div class="i-ri-ghost-line text-3xl mb-2"></div>
        <p>No commands found</p>
        <p class="text-sm">Try a different search term</p>
      </div>
    {:else}
      {@const grouped = keyboardStore.filteredCommands.reduce(
        (acc: Record<string, CommandPaletteItem[]>, item) => {
          if (!acc[item.category]) acc[item.category] = [];
          acc[item.category].push(item);
          return acc;
        },
        {} as Record<string, CommandPaletteItem[]>,
      )}

      {#each Object.entries(grouped) as [category, items], groupIndex}
        <div class="px-4 py-2 text-xs font-semibold text-fg-dim uppercase tracking-wide flex items-center gap-2">
          <span class="{getCategoryIcon(category)} {getCategoryColor(category)}"></span>
          {category}
        </div>
        {#each items as item, index (item.id)}
          {@const globalIndex = keyboardStore.filteredCommands.indexOf(item)}
          <button
            class="w-full px-4 py-3 flex items-center gap-3 text-left transition-colors hover:bg-bg-soft {selectedIndex ===
            globalIndex
              ? 'bg-bg-soft'
              : ''}"
            onclick={() => handleSelect(item)}
            onmouseenter={() => (selectedIndex = globalIndex)}>
            {#if item.icon}
              <span class="{item.icon} text-fg-dim"></span>
            {/if}
            <div class="flex-1 min-w-0">
              <div class="text-sm text-fg font-medium">{item.title}</div>
              {#if item.subtitle}
                <div class="text-xs text-fg-dim truncate">{item.subtitle}</div>
              {/if}
            </div>
            {#if item.shortcut}
              <kbd class="px-2 py-1 bg-bg-muted rounded text-xs text-fg-dim">{item.shortcut}</kbd>
            {/if}
          </button>
        {/each}
      {/each}
    {/if}
  </div>

  <div class="px-4 py-2 border-t border-bg-muted text-xs text-fg-dim flex justify-between">
    <span>{keyboardStore.filteredCommands.length} commands available</span>
    <span>Cmd+K to open</span>
  </div>
</Modal>
