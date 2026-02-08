import { logError } from "./logger.svelte";

type BookmarkKind = "session" | "search" | "filter" | "chart";

type BookmarkData = {
  sessionId?: string;
  eventId?: string;
  query?: string;
  filters?: Record<string, string | null>;
  chartType?: string;
  chartParams?: Record<string, unknown>;
};

export type Bookmark = {
  id: string;
  name: string;
  type: BookmarkKind;
  description?: string;
  timestamp: string;
  data: BookmarkData;
};

const STORAGE_KEY = "agent-v-bookmarks";

function createBookmarkStore() {
  let bookmarks = $state<Bookmark[]>([]);

  function loadFromStorage(): Bookmark[] {
    if (typeof window === "undefined") return [];
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        return JSON.parse(stored);
      }
    } catch {
      logError("Failed to load bookmarks from storage");
    }
    return [];
  }

  function saveToStorage(items: Bookmark[]): void {
    if (typeof window === "undefined") return;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
    } catch {
      logError("Failed to save bookmarks to storage");
    }
  }

  function init(): void {
    bookmarks = loadFromStorage();
  }

  function add(bookmark: Omit<Bookmark, "id" | "timestamp">): void {
    const newBookmark: Bookmark = { ...bookmark, id: crypto.randomUUID(), timestamp: new Date().toISOString() };
    bookmarks = [newBookmark, ...bookmarks];
    saveToStorage(bookmarks);
  }

  function remove(id: string): void {
    bookmarks = bookmarks.filter((b) => b.id !== id);
    saveToStorage(bookmarks);
  }

  function update(id: string, updates: Partial<Bookmark>): void {
    bookmarks = bookmarks.map((b) => (b.id === id ? { ...b, ...updates } : b));
    saveToStorage(bookmarks);
  }

  function clear(): void {
    bookmarks = [];
    saveToStorage([]);
  }

  function exportBookmarks(): string {
    return JSON.stringify(bookmarks, null, 2);
  }

  function importBookmarks(json: string): boolean {
    try {
      const items = JSON.parse(json) as Bookmark[];
      if (Array.isArray(items)) {
        bookmarks = items;
        saveToStorage(items);
        return true;
      }
    } catch {
      logError("Failed to import bookmarks");
    }
    return false;
  }

  init();

  return {
    get bookmarks() {
      return bookmarks;
    },
    init,
    add,
    remove,
    update,
    clear,
    export: exportBookmarks,
    import: importBookmarks,
  };
}

export const bookmarkStore = createBookmarkStore();

export function getBookmarkDescription(bookmark: Bookmark): string {
  switch (bookmark.type) {
    case "session":
      return `Session: ${bookmark.data.sessionId?.slice(0, 8) || "Unknown"}`;
    case "search":
      return bookmark.data.query || "Search";
    case "filter": {
      const filterCount = Object.values(bookmark.data.filters || {}).filter(Boolean).length;
      return `${filterCount} active filter${filterCount !== 1 ? "s" : ""}`;
    }
    case "chart":
      return bookmark.data.chartType || "Chart";
    default:
      return "";
  }
}

export function getBookmarkIcon(type: Bookmark["type"]): string {
  switch (type) {
    case "session":
      return "i-ri-chat-3-line";
    case "search":
      return "i-ri-search-line";
    case "filter":
      return "i-ri-filter-3-line";
    case "chart":
      return "i-ri-bar-chart-line";
    default:
      return "i-ri-bookmark-line";
  }
}

export function getBookmarkColor(type: Bookmark["type"]): string {
  switch (type) {
    case "session":
      return "text-blue";
    case "search":
      return "text-green";
    case "filter":
      return "text-purple";
    case "chart":
      return "text-cyan";
    default:
      return "text-fg-muted";
  }
}
