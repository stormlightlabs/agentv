import { goto } from "$app/navigation";
import { resolve } from "$app/paths";
import { SvelteURLSearchParams } from "svelte/reactivity";

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

class FilterStore {
  private stateValue = $state<FilterState>({ ...defaultState });

  private activeCountValue = $derived(
    [
      this.stateValue.query === "" ? 0 : 1,
      this.stateValue.source === null ? 0 : 1,
      this.stateValue.project === null ? 0 : 1,
      this.stateValue.kind === null ? 0 : 1,
      this.stateValue.role === null ? 0 : 1,
      this.stateValue.tool === null ? 0 : 1,
      this.stateValue.since === null ? 0 : 1,
      this.stateValue.until === null ? 0 : 1,
    ].reduce((a, b) => a + b, 0),
  );

  private hasActiveFiltersValue = $derived(
    this.stateValue.query !== "" ||
      this.stateValue.source !== null ||
      this.stateValue.project !== null ||
      this.stateValue.kind !== null ||
      this.stateValue.role !== null ||
      this.stateValue.tool !== null ||
      this.stateValue.since !== null ||
      this.stateValue.until !== null,
  );

  get state() {
    return this.stateValue;
  }

  set state(newState: FilterState) {
    this.stateValue = newState;
  }

  get activeCount() {
    return this.activeCountValue;
  }

  get hasActiveFilters() {
    return this.hasActiveFiltersValue;
  }

  setFilter<K extends keyof FilterState>(key: K, value: FilterState[K]) {
    this.stateValue = { ...this.stateValue, [key]: value };
  }

  clearFilter<K extends keyof FilterState>(key: K) {
    const clearedValue = (key === "query" ? "" : null) as FilterState[K];
    this.stateValue = { ...this.stateValue, [key]: clearedValue };
  }

  clearAll() {
    this.stateValue = { ...defaultState };
  }
}

export const filterStore = new FilterStore();

export function syncFiltersFromURL(searchParams: URLSearchParams): void {
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
  const searchParams = new SvelteURLSearchParams();

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

  const url = resolve("/", Object.fromEntries(searchParams));

  goto(url, { replaceState: true, keepFocus: true });
}

export function buildDeepLink(filters: FilterState): string {
  const searchParams = new SvelteURLSearchParams();

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
  return queryString ? `${globalThis.location.origin}?${queryString}` : globalThis.location.origin;
}
