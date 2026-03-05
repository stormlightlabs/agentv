<script lang="ts" generics="T extends Record<string, unknown>">
  import type { DataTableColumn, DataTableRowAction } from "$lib/types";
  import { fade, fly } from "svelte/transition";

  type Props = {
    data: T[];
    columns: DataTableColumn<T>[];
    keyExtractor: (row: T) => string;
    loading?: boolean;
    selectable?: boolean;
    pageSize?: number;
    onSelect?: (row: T) => void;
    selectedId?: string | null;
    rowActions?: DataTableRowAction<T>[];
    expandableRows?: boolean;
  };

  let {
    data,
    columns,
    keyExtractor,
    loading = false,
    selectable = true,
    pageSize = 50,
    onSelect,
    selectedId = null,
    rowActions = [],
    expandableRows = false,
  }: Props = $props();

  let sortKey = $state<string | null>(null);
  let sortDirection = $state<"asc" | "desc">("asc");
  let filters = $state<Record<string, string>>({});
  let currentPage = $state(1);
  let expandedRowIds = $state<string[]>([]);

  function handleSort(column: DataTableColumn<T>) {
    if (!column.sortable) return;

    const key = String(column.key);
    if (sortKey === key) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortKey = key;
      sortDirection = "asc";
    }
  }

  function getCellValue(row: T, key: keyof T | string): unknown {
    if (typeof key === "string" && key.includes(".")) {
      const keys = key.split(".");
      let value: unknown = row;
      for (const k of keys) {
        if (value && typeof value === "object") {
          value = (value as Record<string, unknown>)[k];
        } else {
          return undefined;
        }
      }
      return value;
    }
    return row[key as keyof T];
  }

  let filteredData = $derived.by(() => {
    let result = [...data];

    for (const [key, filterValue] of Object.entries(filters)) {
      if (filterValue) {
        const lowerFilter = filterValue.toLowerCase();
        result = result.filter((row) => {
          const value = getCellValue(row, key);
          return String(value ?? "")
            .toLowerCase()
            .includes(lowerFilter);
        });
      }
    }

    if (sortKey) {
      const sortKeyValue = sortKey as keyof T | string;
      result.sort((a, b) => {
        const aVal = getCellValue(a, sortKeyValue);
        const bVal = getCellValue(b, sortKeyValue);

        if (aVal === null || aVal === undefined) return 1;
        if (bVal === null || bVal === undefined) return -1;

        const comparison = String(aVal).localeCompare(String(bVal));
        return sortDirection === "asc" ? comparison : -comparison;
      });
    }

    return result;
  });

  let totalPages = $derived(Math.ceil(filteredData.length / pageSize));
  let paginatedData = $derived(filteredData.slice((currentPage - 1) * pageSize, currentPage * pageSize));
  let hasRowActions = $derived(rowActions.length > 0 || expandableRows);
  let totalColumns = $derived(columns.length + (hasRowActions ? 1 : 0));

  function handlePageChange(page: number) {
    if (page >= 1 && page <= totalPages) {
      currentPage = page;
    }
  }

  function getSortIcon(column: DataTableColumn<T>): string {
    if (sortKey !== String(column.key)) return "i-ri-arrow-up-down-line";
    return sortDirection === "asc" ? "i-ri-arrow-up-line" : "i-ri-arrow-down-line";
  }

  function isRowExpanded(rowId: string): boolean {
    return expandedRowIds.includes(rowId);
  }

  function toggleRowExpansion(rowId: string): void {
    expandedRowIds = isRowExpanded(rowId) ? expandedRowIds.filter((id) => id !== rowId) : [...expandedRowIds, rowId];
  }

  function formatColumnValue(row: T, column: DataTableColumn<T>): string {
    if (column.render) {
      const rendered = column.render(row);
      if (typeof rendered === "string") return rendered;
      return rendered.text;
    }
    const value = getCellValue(row, column.key);
    return String(value ?? "-");
  }

  function getColumnClass(row: T, column: DataTableColumn<T>): string {
    if (column.render) {
      const rendered = column.render(row);
      if (typeof rendered === "object" && rendered.className) {
        return rendered.className;
      }
    }
    return "";
  }

  $effect(() => {
    if (Object.keys(filters).length > 0) {
      currentPage = 1;
    }
  });

  $effect(() => {
    const validIds = new Set(filteredData.map((row) => keyExtractor(row)));
    expandedRowIds = expandedRowIds.filter((id) => validIds.has(id));
  });
</script>

<div class="flex flex-col h-full overflow-hidden">
  {#if columns.some((c) => c.filterable)}
    <div class="p-2 border-b border-surface-muted bg-surface-soft flex flex-wrap gap-2">
      {#each columns.filter((c) => c.filterable) as column (String(column.key))}
        <div class="flex items-center gap-1">
          <input
            type="text"
            class="px-2 py-1 bg-surface border border-surface-muted rounded text-xs text-fg placeholder-fg-dim focus:outline-none focus:border-blue"
            placeholder="Filter {column.header}..."
            value={filters[String(column.key)] || ""}
            oninput={(e) => {
              filters = { ...filters, [String(column.key)]: e.currentTarget.value };
            }} />
        </div>
      {/each}
      {#if Object.keys(filters).length > 0}
        <button class="px-2 py-1 text-xs text-fg-dim hover:text-fg transition-colors" onclick={() => (filters = {})}>
          Clear filters
        </button>
      {/if}
    </div>
  {/if}

  <div class="flex-1 overflow-auto">
    {#if loading}
      <div class="p-4 space-y-2">
        {#each Array.from({ length: 5 }) as _, i (i)}
          <div class="h-12 bg-surface-muted rounded animate-pulse" in:fade={{ delay: i * 50 }}></div>
        {/each}
      </div>
    {:else if paginatedData.length === 0}
      <div class="flex items-center justify-center h-full text-fg-dim p-8">
        <div class="text-center" in:fade>
          <div class="i-ri-inbox-line text-4xl mb-2 mx-auto"></div>
          <p class="text-sm">No data available</p>
        </div>
      </div>
    {:else}
      <table class="w-full text-sm">
        <thead class="sticky top-0 bg-surface-soft z-10">
          <tr class="border-b border-surface-muted">
            {#each columns as column (String(column.key))}
              <th
                class="px-4 py-2 text-left text-xs font-semibold text-fg-dim uppercase tracking-wide whitespace-nowrap {column.sortable
                  ? 'cursor-pointer hover:text-fg select-none'
                  : ''}"
                style={column.width ? `width: ${column.width}` : undefined}
                onclick={() => handleSort(column)}>
                <div class="flex items-center gap-1">
                  {column.header}
                  {#if column.sortable}
                    <span class="{getSortIcon(column)} text-xs opacity-50"></span>
                  {/if}
                </div>
              </th>
            {/each}
            {#if hasRowActions}
              <th class="px-4 py-2 text-right text-xs font-semibold text-fg-dim uppercase tracking-wide whitespace-nowrap">
                Actions
              </th>
            {/if}
          </tr>
        </thead>
        <tbody>
          {#each paginatedData as row, index (keyExtractor(row))}
            {@const rowId = keyExtractor(row)}
            <tr
              class="border-b border-surface-muted last:border-b-0 transition-colors {selectable
                ? 'cursor-pointer hover:bg-surface-muted'
                : ''} {selectedId === rowId ? 'bg-surface-muted' : ''}"
              in:fly={{ y: 10, duration: 200, delay: index * 30 }}
              onclick={() => selectable && onSelect?.(row)}>
              {#each columns as column (String(column.key))}
                <td class="px-4 py-3 {getColumnClass(row, column)}">
                  <div class="max-w-[320px] truncate whitespace-nowrap" title={formatColumnValue(row, column)}>
                    {formatColumnValue(row, column)}
                  </div>
                </td>
              {/each}
              {#if hasRowActions}
                <td class="px-4 py-3">
                  <div class="flex justify-end gap-1">
                    {#each rowActions as action (action.id)}
                      <button
                        class="bg-surface border-surface-muted text-fg-dim hover:text-fg rounded border px-2 py-1 text-xs"
                        title={action.title ?? action.label}
                        type="button"
                        onclick={(event) => {
                          event.stopPropagation();
                          action.onClick(row);
                        }}>
                        {#if action.icon}
                          <span class={action.icon}></span>
                        {/if}
                        <span>{action.label}</span>
                      </button>
                    {/each}
                    {#if expandableRows}
                      <button
                        class="bg-surface border-surface-muted text-fg-dim hover:text-fg rounded border px-2 py-1 text-xs"
                        title={isRowExpanded(rowId) ? "Collapse row details" : "Expand row details"}
                        type="button"
                        onclick={(event) => {
                          event.stopPropagation();
                          toggleRowExpansion(rowId);
                        }}>
                        <span class={isRowExpanded(rowId) ? "i-ri-arrow-up-s-line" : "i-ri-arrow-down-s-line"}></span>
                        <span>{isRowExpanded(rowId) ? "Collapse" : "Expand"}</span>
                      </button>
                    {/if}
                  </div>
                </td>
              {/if}
            </tr>
            {#if expandableRows && isRowExpanded(rowId)}
              <tr class="border-b border-surface-muted bg-surface/50">
                <td class="px-4 py-3" colspan={totalColumns}>
                  <div class="grid gap-3 sm:grid-cols-2">
                    {#each columns as column (String(column.key))}
                      <div class="min-w-0">
                        <div class="text-fg-dim text-2xs uppercase tracking-wide">{column.header}</div>
                        <div class="text-fg mt-1 break-words whitespace-pre-wrap">{formatColumnValue(row, column)}</div>
                      </div>
                    {/each}
                  </div>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    {/if}
  </div>

  {#if totalPages > 1}
    <div class="p-2 border-t border-surface-muted bg-surface-soft flex items-center justify-between text-xs">
      <span class="text-fg-dim">
        Showing {(currentPage - 1) * pageSize + 1} - {Math.min(currentPage * pageSize, filteredData.length)} of {filteredData.length}
      </span>
      <div class="flex items-center gap-1">
        <button
          class="px-2 py-1 rounded border border-surface-muted bg-surface text-fg-dim hover:text-fg disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          onclick={() => handlePageChange(currentPage - 1)}
          disabled={currentPage === 1}
          aria-label="Previous page"
          title="Previous page">
          <span class="i-ri-arrow-left-s-line"></span>
        </button>
        <span class="px-2 text-fg-dim">
          {currentPage} / {totalPages}
        </span>
        <button
          class="px-2 py-1 rounded border border-surface-muted bg-surface text-fg-dim hover:text-fg disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          onclick={() => handlePageChange(currentPage + 1)}
          disabled={currentPage === totalPages}
          aria-label="Next page"
          title="Next page">
          <span class="i-ri-arrow-right-s-line"></span>
        </button>
      </div>
    </div>
  {/if}
</div>
