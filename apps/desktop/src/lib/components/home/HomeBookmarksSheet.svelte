<script lang="ts">
  import Sheet from "$lib/components/Sheet.svelte";
  import {
    getBookmarkColor,
    getBookmarkIcon,
    type Bookmark,
  } from "$lib/stores/bookmarks.svelte";

  type Props = {
    open: boolean;
    bookmarks: Bookmark[];
    onOpenChange?: (open: boolean) => void;
    onApplyBookmark?: (bookmark: Bookmark) => void;
    onDeleteBookmark?: (id: string) => void;
  };

  let { open, bookmarks, onOpenChange, onApplyBookmark, onDeleteBookmark }: Props = $props();

  function closeSheet() {
    onOpenChange?.(false);
  }
</script>

<Sheet {open} {onOpenChange} side="right" width="md" aria-label="Bookmarks">
  <div class="flex h-full flex-col">
    <div class="border-surface-muted flex items-center justify-between border-b p-4">
      <h2 class="text-fg m-0 text-lg font-semibold">Bookmarks</h2>
      <button
        class="text-fg-dim hover:text-fg p-2 transition-colors"
        onclick={closeSheet}
        aria-label="Close bookmarks"
        type="button">
        <span class="i-ri-close-line"></span>
      </button>
    </div>

    <div class="flex-1 space-y-2 overflow-y-auto p-4">
      {#if bookmarks.length === 0}
        <div class="text-fg-dim py-8 text-center">
          <div class="i-ri-bookmark-line mb-2 text-3xl opacity-50"></div>
          <p>No bookmarks yet</p>
          <p class="text-sm">Use Cmd+D to bookmark sessions</p>
        </div>
      {:else}
        {#each bookmarks as bookmark (bookmark.id)}
          <div
            class="group bg-surface-soft border-surface-muted hover:border-blue flex items-start gap-3 rounded border p-3 transition-colors"
            onclick={() => onApplyBookmark?.(bookmark)}
            role="button"
            tabindex="0"
            onkeydown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onApplyBookmark?.(bookmark);
              }
            }}>
            <span class="{getBookmarkIcon(bookmark.type)} {getBookmarkColor(bookmark.type)} mt-0.5"></span>
            <div class="min-w-0 flex-1">
              <div class="text-fg truncate text-sm font-medium">{bookmark.name}</div>
              {#if bookmark.description}
                <div class="text-fg-dim truncate text-xs">{bookmark.description}</div>
              {/if}
            </div>
            <button
              class="text-fg-dim hover:text-red p-1 opacity-0 transition-all group-hover:opacity-100"
              onclick={(event) => {
                event.stopPropagation();
                onDeleteBookmark?.(bookmark.id);
              }}
              aria-label="Delete bookmark"
              type="button">
              <span class="i-ri-delete-bin-line"></span>
            </button>
          </div>
        {/each}
      {/if}
    </div>
  </div>
</Sheet>
