<script lang="ts">
  import { keyboardStore, type CommandPaletteItem } from "$lib/stores/keyboard.svelte";
  import { onMount } from "svelte";
  import Modal from "./Modal.svelte";

  let selectedIndex = $derived(0);
  let inputRef = $state<HTMLInputElement | null>(null);

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
      case "navigation": {
        return "i-ri-compass-3-line";
      }
      case "action": {
        return "i-ri-flashlight-line";
      }
      case "search": {
        return "i-ri-search-line";
      }
      case "export": {
        return "i-ri-download-line";
      }
      case "view": {
        return "i-ri-eye-line";
      }
      default: {
        return "i-ri-circle-fill";
      }
    }
  }

  function getCategoryColor(category: string): string {
    switch (category) {
      case "navigation": {
        return "text-blue";
      }
      case "action": {
        return "text-yellow";
      }
      case "search": {
        return "text-green";
      }
      case "export": {
        return "text-purple";
      }
      case "view": {
        return "text-cyan";
      }
      default: {
        return "text-fg-muted";
      }
    }
  }

  function keydownHandler(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      keyboardStore.openCommandPalette();
    }
  }

  onMount(() => {
    globalThis.addEventListener("keydown", keydownHandler);
    return () => globalThis.removeEventListener("keydown", keydownHandler);
  });

  $effect(() => {
    if (keyboardStore.commandPaletteOpen) {
      selectedIndex = 0;
      inputRef?.focus();
    }
  });
</script>

<Modal
  bind:open={keyboardStore.commandPaletteOpen}
  size="md"
  class="items-start! pt-[20vh]!"
  contentClass="border border-surface-muted"
  aria-label="Command palette">
  <div class="border-surface-muted flex items-center gap-3 border-b px-4 py-3">
    <span class="i-ri-command-line text-fg-muted"></span>
    <input
      bind:this={inputRef}
      type="text"
      class="text-fg font-inherit placeholder-fg-muted flex-1 border-none bg-transparent text-base focus:outline-none"
      placeholder="Type a command or search..."
      bind:value={keyboardStore.commandPaletteSearch}
      onkeydown={handleKeydown} />
    <div class="flex gap-1">
      <kbd class="bg-surface-muted text-fg-dim rounded px-2 py-1 text-xs">↑↓</kbd>
      <kbd class="bg-surface-muted text-fg-dim rounded px-2 py-1 text-xs">↵</kbd>
      <kbd class="bg-surface-muted text-fg-dim rounded px-2 py-1 text-xs">Esc</kbd>
    </div>
  </div>

  <div class="max-h-[50vh] overflow-y-auto">
    {#if keyboardStore.filteredCommands.length === 0}
      <div class="text-fg-dim px-4 py-8 text-center">
        <div class="i-ri-ghost-line mb-2 text-3xl"></div>
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

      {#each Object.entries(grouped) as [category, items] (category)}
        <div class="text-fg-dim flex items-center gap-2 px-4 py-2 text-xs font-semibold tracking-wide uppercase">
          <span class="{getCategoryIcon(category)} {getCategoryColor(category)}"></span>
          {category}
        </div>
        {#each items as item (item.id)}
          {@const globalIndex = keyboardStore.filteredCommands.indexOf(item)}
          <button
            class="hover:bg-surface-soft flex w-full items-center gap-3 px-4 py-3 text-left transition-colors {selectedIndex ===
            globalIndex
              ? 'bg-surface-soft'
              : ''}"
            onclick={() => handleSelect(item)}
            onmouseenter={() => (selectedIndex = globalIndex)}>
            {#if item.icon}
              <span class="{item.icon} text-fg-dim"></span>
            {/if}
            <div class="min-w-0 flex-1">
              <div class="text-fg text-sm font-medium">{item.title}</div>
              {#if item.subtitle}
                <div class="text-fg-dim truncate text-xs">{item.subtitle}</div>
              {/if}
            </div>
            {#if item.shortcut}
              <kbd class="bg-surface-muted text-fg-dim rounded px-2 py-1 text-xs">{item.shortcut}</kbd>
            {/if}
          </button>
        {/each}
      {/each}
    {/if}
  </div>

  <div class="border-surface-muted text-fg-dim flex justify-between border-t px-4 py-2 text-xs">
    <span>{keyboardStore.filteredCommands.length} commands available</span>
    <span>Cmd+K to open</span>
  </div>
</Modal>
