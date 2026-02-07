type ShortcutScope = "global" | "search" | "viewer";
type ShortcutCategory = "navigation" | "action" | "search" | "export" | "view";
type KeyboardModifiers = { ctrl?: boolean; alt?: boolean; shift?: boolean; meta?: boolean };

export type KeyboardShortcut = {
  key: string;
  modifiers?: KeyboardModifiers;
  description: string;
  handler: () => void;
  scope?: ShortcutScope;
};

export type CommandPaletteItem = {
  id: string;
  title: string;
  subtitle?: string;
  icon?: string;
  shortcut?: string;
  category: ShortcutCategory;
  action: () => void;
};

function createKeyboardStore() {
  let commandPaletteOpen = $state(false);
  let commandPaletteItems = $state<CommandPaletteItem[]>([]);
  let commandPaletteSearch = $state("");

  const filteredCommands = $derived(
    commandPaletteSearch === ""
      ? commandPaletteItems
      : commandPaletteItems.filter((item) => {
          const query = commandPaletteSearch.toLowerCase();
          return (
            item.title.toLowerCase().includes(query) ||
            item.subtitle?.toLowerCase().includes(query) ||
            item.category.toLowerCase().includes(query)
          );
        }),
  );

  function openCommandPalette() {
    commandPaletteOpen = true;
    commandPaletteSearch = "";
  }

  function closeCommandPalette() {
    commandPaletteOpen = false;
  }

  function setCommandPaletteItems(items: CommandPaletteItem[]) {
    commandPaletteItems = items;
  }

  return {
    get commandPaletteOpen() {
      return commandPaletteOpen;
    },
    set commandPaletteOpen(value) {
      commandPaletteOpen = value;
    },
    get commandPaletteItems() {
      return commandPaletteItems;
    },
    get commandPaletteSearch() {
      return commandPaletteSearch;
    },
    set commandPaletteSearch(value) {
      commandPaletteSearch = value;
    },
    get filteredCommands() {
      return filteredCommands;
    },
    openCommandPalette,
    closeCommandPalette,
    setCommandPaletteItems,
  };
}

export const keyboardStore = createKeyboardStore();

const shortcuts: KeyboardShortcut[] = [];

export function registerShortcut(shortcut: KeyboardShortcut): () => void {
  shortcuts.push(shortcut);
  return () => {
    const index = shortcuts.indexOf(shortcut);
    if (index > -1) shortcuts.splice(index, 1);
  };
}

export function getShortcuts(): KeyboardShortcut[] {
  return [...shortcuts];
}

export function handleKeyboardEvent(event: KeyboardEvent): boolean {
  const shortcut = shortcuts.find((s) => matchesShortcut(event, s));

  if (shortcut) {
    event.preventDefault();
    shortcut.handler();
    return true;
  }

  return false;
}

function matchesShortcut(event: KeyboardEvent, shortcut: KeyboardShortcut): boolean {
  const keyMatches = event.key.toLowerCase() === shortcut.key.toLowerCase();
  const ctrlMatches = !!shortcut.modifiers?.ctrl === event.ctrlKey;
  const altMatches = !!shortcut.modifiers?.alt === event.altKey;
  const shiftMatches = !!shortcut.modifiers?.shift === event.shiftKey;
  const metaMatches = !!shortcut.modifiers?.meta === event.metaKey;

  return keyMatches && ctrlMatches && altMatches && shiftMatches && metaMatches;
}

export function formatShortcut(shortcut: KeyboardShortcut): string {
  const parts: string[] = [];
  if (shortcut.modifiers?.meta) parts.push("Cmd");
  if (shortcut.modifiers?.ctrl) parts.push("Ctrl");
  if (shortcut.modifiers?.alt) parts.push("Alt");
  if (shortcut.modifiers?.shift) parts.push("Shift");
  parts.push(shortcut.key.toUpperCase());
  return parts.join("+");
}
