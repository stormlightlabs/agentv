import { goto } from "$app/navigation";
import { page } from "$app/state";

type Nullable<T> = T | null;

export type FilterState = {
  query: string;
  source: Nullable<string>;
  project: Nullable<string>;
  kind: Nullable<string>;
  role: Nullable<string>;
  tool: Nullable<string>;
  since: Nullable<string>;
  until: Nullable<string>;
  sessionId: Nullable<string>;
  eventId: Nullable<string>;
};

const defaultState: FilterState = {
  query: "",
  source: null,
  project: null,
  kind: null,
  role: null,
  tool: null,
  since: null,
  until: null,
  sessionId: null,
  eventId: null,
};

function createFilterStore() {
  let state = $state<FilterState>({ ...defaultState });

  const activeCount = $derived(
    [
      state.query !== "" ? 1 : 0,
      state.source !== null ? 1 : 0,
      state.project !== null ? 1 : 0,
      state.kind !== null ? 1 : 0,
      state.role !== null ? 1 : 0,
      state.tool !== null ? 1 : 0,
      state.since !== null ? 1 : 0,
      state.until !== null ? 1 : 0,
    ].reduce((a, b) => a + b, 0),
  );

  const hasActiveFilters = $derived(
    state.query !== "" ||
      state.source !== null ||
      state.project !== null ||
      state.kind !== null ||
      state.role !== null ||
      state.tool !== null ||
      state.since !== null ||
      state.until !== null,
  );

  function setFilter(key: keyof FilterState, value: string | null) {
    state = { ...state, [key]: value };
  }

  function clearFilter(key: keyof FilterState) {
    state = { ...state, [key]: null };
  }

  function clearAll() {
    state = { ...defaultState };
  }

  return {
    get state() {
      return state;
    },
    set state(newState) {
      state = newState;
    },
    get activeCount() {
      return activeCount;
    },
    get hasActiveFilters() {
      return hasActiveFilters;
    },
    setFilter,
    clearFilter,
    clearAll,
  };
}

export const filterStore = createFilterStore();

export function syncFiltersFromURL(): void {
  const searchParams = new URLSearchParams(page.url.searchParams);

  filterStore.state = {
    query: searchParams.get("q") || "",
    source: searchParams.get("source"),
    project: searchParams.get("project"),
    kind: searchParams.get("kind"),
    role: searchParams.get("role"),
    tool: searchParams.get("tool"),
    since: searchParams.get("since"),
    until: searchParams.get("until"),
    sessionId: searchParams.get("session"),
    eventId: searchParams.get("event"),
  };
}

export function updateURLFromFilters(filters: FilterState): void {
  const searchParams = new URLSearchParams();

  if (filters.query) searchParams.set("q", filters.query);
  if (filters.source) searchParams.set("source", filters.source);
  if (filters.project) searchParams.set("project", filters.project);
  if (filters.kind) searchParams.set("kind", filters.kind);
  if (filters.role) searchParams.set("role", filters.role);
  if (filters.tool) searchParams.set("tool", filters.tool);
  if (filters.since) searchParams.set("since", filters.since);
  if (filters.until) searchParams.set("until", filters.until);
  if (filters.sessionId) searchParams.set("session", filters.sessionId);
  if (filters.eventId) searchParams.set("event", filters.eventId);

  const queryString = searchParams.toString();
  const url = queryString ? `?${queryString}` : "/";

  goto(url, { replaceState: true, keepFocus: true });
}

export function buildDeepLink(filters: FilterState): string {
  const searchParams = new URLSearchParams();

  if (filters.query) searchParams.set("q", filters.query);
  if (filters.source) searchParams.set("source", filters.source);
  if (filters.project) searchParams.set("project", filters.project);
  if (filters.kind) searchParams.set("kind", filters.kind);
  if (filters.role) searchParams.set("role", filters.role);
  if (filters.tool) searchParams.set("tool", filters.tool);
  if (filters.since) searchParams.set("since", filters.since);
  if (filters.until) searchParams.set("until", filters.until);
  if (filters.sessionId) searchParams.set("session", filters.sessionId);
  if (filters.eventId) searchParams.set("event", filters.eventId);

  const queryString = searchParams.toString();
  return queryString ? `${window.location.origin}?${queryString}` : window.location.origin;
}
