<script lang="ts" generics="T extends Record<string, unknown>">
  import type { DataTableColumn, DataTableRowAction } from "$lib/types";
  import { onMount } from "svelte";
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
  let tableViewportEl = $state<HTMLDivElement | null>(null);
  let tableViewportWidth = $state(0);
  let columnWidths = $state<Record<string, number>>({});
  let hasUserResizedColumns = $state(false);
  let resizingColumn = $state<{ key: string; startX: number; startWidth: number } | null>(null);
  const minColumnWidth = 120;
  const actionColumnWidth = 112;
  let validExpandedRowIds = $derived.by(() => {
    const validIds = new Set(filteredData.map((row) => keyExtractor(row)));
    return expandedRowIds.filter((id) => validIds.has(id));
  });

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
  let columnKeys = $derived(columns.map((column) => String(column.key)));

  function handlePageChange(page: number) {
    if (page >= 1 && page <= totalPages) {
      currentPage = page;
    }
  }

  function getSortIcon(column: DataTableColumn<T>): string {
    if (sortKey !== String(column.key)) return "i-ri-arrow-up-down-line";
    return sortDirection === "asc" ? "i-ri-arrow-up-line" : "i-ri-arrow-down-line";
  }

  function getColumnWidth(column: DataTableColumn<T>): number {
    return columnWidths[String(column.key)] ?? getInitialColumnWidth(column);
  }

  function getInitialColumnWidth(column: DataTableColumn<T>): number {
    if (!column.width) return minColumnWidth;
    const parsed = Number.parseInt(column.width, 10);
    if (Number.isNaN(parsed) || parsed <= 0) return minColumnWidth;
    return Math.max(minColumnWidth, parsed);
  }

  function startColumnResize(event: MouseEvent, column: DataTableColumn<T>): void {
    event.preventDefault();
    event.stopPropagation();
    const key = String(column.key);
    hasUserResizedColumns = true;
    resizingColumn = { key, startX: event.clientX, startWidth: columnWidths[key] ?? getColumnWidth(column) };
  }

  function nudgeColumnWidth(column: DataTableColumn<T>, delta: number): void {
    const key = String(column.key);
    hasUserResizedColumns = true;
    const currentWidth = columnWidths[key] ?? getColumnWidth(column);
    columnWidths = { ...columnWidths, [key]: Math.max(minColumnWidth, currentWidth + delta) };
  }

  function resizeColumn(event: MouseEvent): void {
    if (!resizingColumn) return;
    const delta = event.clientX - resizingColumn.startX;
    const nextWidth = Math.max(minColumnWidth, resizingColumn.startWidth + delta);
    columnWidths = { ...columnWidths, [resizingColumn.key]: nextWidth };
  }

  function stopColumnResize(): void {
    resizingColumn = null;
  }

  function isRowExpanded(rowId: string): boolean {
    return validExpandedRowIds.includes(rowId);
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
    if (hasUserResizedColumns || columnKeys.length === 0 || tableViewportWidth <= 0) {
      return;
    }

    const availableWidth = Math.max(
      tableViewportWidth - (hasRowActions ? actionColumnWidth : 0),
      minColumnWidth * columnKeys.length,
    );
    const equalWidth = Math.max(minColumnWidth, Math.floor(availableWidth / columnKeys.length));
    columnWidths = Object.fromEntries(
      columns.map((column) => [String(column.key), Math.max(equalWidth, getInitialColumnWidth(column))]),
    );
  });

  onMount(() => {
    const updateTableWidth = () => {
      if (tableViewportEl) {
        tableViewportWidth = tableViewportEl.clientWidth;
      }
    };

    const observer = new ResizeObserver(updateTableWidth);
    if (tableViewportEl) {
      observer.observe(tableViewportEl);
    }
    updateTableWidth();

    globalThis.addEventListener("mousemove", resizeColumn);
    globalThis.addEventListener("mouseup", stopColumnResize);

    return () => {
      observer.disconnect();
      globalThis.removeEventListener("mousemove", resizeColumn);
      globalThis.removeEventListener("mouseup", stopColumnResize);
    };
  });
</script>

<div class="flex h-full flex-col overflow-hidden">
  {#if columns.some((c) => c.filterable)}
    <div class="border-surface-muted bg-surface-soft flex flex-wrap gap-2 border-b p-2">
      {#each columns.filter((c) => c.filterable) as column (String(column.key))}
        <div class="flex items-center gap-1">
          <input
            type="text"
            class="bg-surface border-surface-muted text-fg placeholder-fg-dim focus:border-blue rounded border px-2 py-1 text-xs focus:outline-none"
            placeholder="Filter {column.header}..."
            value={filters[String(column.key)] || ""}
            oninput={(e) => {
              filters = { ...filters, [String(column.key)]: e.currentTarget.value };
            }} />
        </div>
      {/each}
      {#if Object.keys(filters).length > 0}
        <button class="text-fg-dim hover:text-fg px-2 py-1 text-xs transition-colors" onclick={() => (filters = {})}>
          Clear filters
        </button>
      {/if}
    </div>
  {/if}

  <div class="flex-1 overflow-auto" bind:this={tableViewportEl}>
    {#if loading}
      <div class="space-y-2 p-4">
        {#each Array.from({ length: 5 }) as _, i (i)}
          <div class="bg-surface-muted h-12 animate-pulse rounded" in:fade={{ delay: i * 50 }}></div>
        {/each}
      </div>
    {:else if paginatedData.length === 0}
      <div class="text-fg-dim flex h-full items-center justify-center p-8">
        <div class="text-center" in:fade>
          <div class="i-ri-inbox-line mx-auto mb-2 text-4xl"></div>
          <p class="text-sm">No data available</p>
        </div>
      </div>
    {:else}
      <table class="w-full table-fixed text-sm">
        <thead class="bg-surface-soft sticky top-0 z-10">
          <tr class="border-surface-muted border-b">
            {#each columns as column (String(column.key))}
              <th
                class="text-fg-dim relative px-4 py-2 pr-5 text-left text-xs font-semibold tracking-wide whitespace-nowrap uppercase {column.sortable
                  ? 'hover:text-fg cursor-pointer select-none'
                  : ''}"
                style={`width: ${getColumnWidth(column)}px;`}
                onclick={() => handleSort(column)}>
                <div class="flex items-center gap-1">
                  {column.header}
                  {#if column.sortable}
                    <span class="{getSortIcon(column)} text-xs opacity-50"></span>
                  {/if}
                </div>
                <button
                  class="absolute top-0 right-0 h-full w-2 cursor-col-resize select-none"
                  type="button"
                  aria-label="Resize {column.header} column"
                  onclick={(event) => event.stopPropagation()}
                  onkeydown={(event) => {
                    if (event.key === "ArrowLeft") {
                      event.preventDefault();
                      event.stopPropagation();
                      nudgeColumnWidth(column, -12);
                    } else if (event.key === "ArrowRight") {
                      event.preventDefault();
                      event.stopPropagation();
                      nudgeColumnWidth(column, 12);
                    }
                  }}
                  onmousedown={(event) => startColumnResize(event, column)}>
                </button>
              </th>
            {/each}
            {#if hasRowActions}
              <th
                class="text-fg-dim px-4 py-2 text-right text-xs font-semibold tracking-wide whitespace-nowrap uppercase"
                style={`width: ${actionColumnWidth}px;`}>
                Actions
              </th>
            {/if}
          </tr>
        </thead>
        <tbody>
          {#each paginatedData as row, index (keyExtractor(row))}
            {@const rowId = keyExtractor(row)}
            <tr
              class="border-surface-muted border-b transition-colors last:border-b-0 {selectable
                ? 'hover:bg-surface-muted cursor-pointer'
                : ''} {selectedId === rowId ? 'bg-surface-muted' : ''}"
              in:fly={{ y: 10, duration: 200, delay: index * 30 }}
              onclick={() => selectable && onSelect?.(row)}>
              {#each columns as column (String(column.key))}
                <td class="px-4 py-3 {getColumnClass(row, column)}">
                  <div class="min-w-0 truncate whitespace-nowrap" title={formatColumnValue(row, column)}>
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
                        <span class:sr-only={!!action.icon}>{action.label}</span>
                      </button>
                    {/each}
                    {#if expandableRows}
                      <button
                        class="bg-surface border-surface-muted text-fg-dim hover:text-fg flex items-center rounded border px-2 py-1 text-xs"
                        title={isRowExpanded(rowId) ? "Collapse row details" : "Expand row details"}
                        type="button"
                        onclick={(event) => {
                          event.stopPropagation();
                          toggleRowExpansion(rowId);
                        }}>
                        <span class={isRowExpanded(rowId) ? "i-ri-arrow-up-s-line" : "i-ri-arrow-down-s-line"}></span>
                        <span class="sr-only">{isRowExpanded(rowId) ? "Collapse" : "Expand"}</span>
                      </button>
                    {/if}
                  </div>
                </td>
              {/if}
            </tr>
            {#if expandableRows && isRowExpanded(rowId)}
              <tr class="border-surface-muted bg-surface/50 border-b">
                <td class="px-4 py-3" colspan={totalColumns}>
                  <div class="grid gap-3 sm:grid-cols-2">
                    {#each columns as column (String(column.key))}
                      <div class="min-w-0">
                        <div class="text-fg-dim text-2xs tracking-wide uppercase">{column.header}</div>
                        <div class="text-fg mt-1 wrap-break-word whitespace-pre-wrap">
                          {formatColumnValue(row, column)}
                        </div>
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
    <div class="border-surface-muted bg-surface-soft flex items-center justify-between border-t p-2 text-xs">
      <span class="text-fg-dim">
        Showing {(currentPage - 1) * pageSize + 1} - {Math.min(currentPage * pageSize, filteredData.length)} of {filteredData.length}
      </span>
      <div class="flex items-center gap-1">
        <button
          class="border-surface-muted bg-surface text-fg-dim hover:text-fg flex items-center rounded border px-2 py-1 transition-colors disabled:cursor-not-allowed disabled:opacity-50"
          onclick={() => handlePageChange(currentPage - 1)}
          disabled={currentPage === 1}
          aria-label="Previous page"
          title="Previous page">
          <i class="i-ri-arrow-left-s-line"></i>
        </button>
        <span class="text-fg-dim px-2">
          {currentPage} / {totalPages}
        </span>
        <button
          class="border-surface-muted bg-surface text-fg-dim hover:text-fg flex items-center rounded border px-2 py-1 transition-colors disabled:cursor-not-allowed disabled:opacity-50"
          onclick={() => handlePageChange(currentPage + 1)}
          disabled={currentPage === totalPages}
          aria-label="Next page"
          title="Next page">
          <i class="i-ri-arrow-right-s-line"></i>
        </button>
      </div>
    </div>
  {/if}
</div>
