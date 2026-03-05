<script lang="ts">
  import type { EventData, EventPayload } from "$lib/types";

  type Props = {
    event: EventData;
    onCopyId?: (id: string) => void;
    onCopyPayload?: (payload: EventPayload) => void;
    onNavigateParent?: (parentUuid: string | null) => void;
  };

  let { event, onCopyId, onCopyPayload, onNavigateParent }: Props = $props();

  let activeTab = $state<"normalized" | "raw" | "thinking" | "tools">("normalized");

  const payload = $derived(event.raw_payload);
  const normalizedFields = $derived(getNormalizedFields(event));
  const toolCalls = $derived(payload ? getToolCalls(payload) : []);
  const thinkingContent = $derived(payload ? getThinkingContent(payload) : null);
  const signature = $derived(payload ? getSignature(payload) : null);

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text).catch(() => {});
  }

  function formatJson(obj: unknown): string {
    return JSON.stringify(obj, null, 2);
  }

  function getNormalizedFields(event: EventData): Array<{ label: string; value: string; type?: string }> {
    const fields: Array<{ label: string; value: string; type?: string }> = [
      { label: "ID", value: event.id },
      { label: "Session ID", value: event.session_id },
      { label: "Kind", value: event.kind },
      { label: "Role", value: event.role || "N/A" },
      { label: "Timestamp", value: new Date(event.timestamp).toLocaleString() },
    ];

    if (payload?.uuid) {
      fields.push({ label: "UUID", value: payload.uuid });
    }
    if (payload?.parentUuid) {
      fields.push({ label: "Parent UUID", value: payload.parentUuid, type: "parent" });
    }
    if (payload?.type) {
      fields.push({ label: "Type", value: payload.type });
    }
    if (payload?.gitBranch) {
      fields.push({ label: "Git Branch", value: payload.gitBranch, type: "git" });
    }
    if (payload?.cwd) {
      fields.push({ label: "Working Directory", value: payload.cwd, type: "path" });
    }

    return fields;
  }

  function getToolCalls(payload: EventPayload): Array<{ id: string; name: string; input: Record<string, unknown> }> {
    const message = payload.message;
    if (!message || !Array.isArray(message.content)) return [];

    return message.content
      .filter(
        (block): block is { type: "tool_use"; id: string; name: string; input: Record<string, unknown> } =>
          block.type === "tool_use" &&
          typeof (block as { id?: string }).id === "string" &&
          typeof (block as { name?: string }).name === "string",
      )
      .map((block) => ({ id: block.id, name: block.name, input: block.input || {} }));
  }

  function getThinkingContent(payload: EventPayload): string | null {
    const message = payload.message;
    if (!message || !Array.isArray(message.content)) return null;

    const thinkingBlock = message.content.find(
      (block): block is { type: "thinking"; thinking: string } =>
        block.type === "thinking" && typeof (block as { thinking?: string }).thinking === "string",
    );

    return thinkingBlock?.thinking ?? null;
  }

  function getSignature(payload: EventPayload): string | null {
    const message = payload.message;
    if (!message || !Array.isArray(message.content)) return null;

    const thinkingBlock = message.content.find(
      (block): block is { type: "thinking"; signature?: string } => block.type === "thinking",
    );

    return thinkingBlock?.signature ?? null;
  }

  function getKindColor(kind: string): string {
    switch (kind) {
      case "error": {
        return "bg-red text-surface";
      }
      case "tool_call": {
        return "bg-purple text-surface";
      }
      case "tool_result": {
        return "bg-green text-surface";
      }
      default: {
        return "bg-surface-muted text-fg";
      }
    }
  }
</script>

<div class="bg-surface flex h-full flex-col">
  <div class="border-surface-muted flex items-center justify-between border-b px-4 py-3">
    <div class="flex items-center gap-2">
      <span class="text-2xs rounded px-2 py-1 font-semibold uppercase {getKindColor(event.kind)}">
        {event.kind}
      </span>
      {#if event.role}
        <span class="text-fg-dim text-xs">{event.role}</span>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <button
        class="text-fg-dim hover:text-fg p-1.5 transition-colors"
        title="Copy ID"
        onclick={() => {
          copyToClipboard(event.id);
          onCopyId?.(event.id);
        }}>
        <span class="i-ri-clipboard-line"></span>
      </button>
      <button
        class="text-fg-dim hover:text-fg p-1.5 transition-colors"
        title="Copy Payload"
        onclick={() => {
          copyToClipboard(formatJson(payload));
          if (payload) onCopyPayload?.(payload);
        }}>
        <span class="i-ri-file-copy-line"></span>
      </button>
    </div>
  </div>

  <div class="border-surface-muted flex border-b">
    <button
      class="px-4 py-2 text-sm transition-colors {activeTab === 'normalized'
        ? 'text-fg border-blue border-b-2'
        : 'text-fg-dim hover:text-fg'}"
      onclick={() => (activeTab = "normalized")}>
      Fields
    </button>
    {#if thinkingContent}
      <button
        class="px-4 py-2 text-sm transition-colors {activeTab === 'thinking'
          ? 'text-fg border-blue border-b-2'
          : 'text-fg-dim hover:text-fg'}"
        onclick={() => (activeTab = "thinking")}>
        Thinking
      </button>
    {/if}
    {#if toolCalls.length > 0}
      <button
        class="px-4 py-2 text-sm transition-colors {activeTab === 'tools'
          ? 'text-fg border-blue border-b-2'
          : 'text-fg-dim hover:text-fg'}"
        onclick={() => (activeTab = "tools")}>
        Tools ({toolCalls.length})
      </button>
    {/if}
    <button
      class="px-4 py-2 text-sm transition-colors {activeTab === 'raw'
        ? 'text-fg border-blue border-b-2'
        : 'text-fg-dim hover:text-fg'}"
      onclick={() => (activeTab = "raw")}>
      Raw JSON
    </button>
  </div>

  <div class="flex-1 overflow-auto p-4">
    {#if activeTab === "normalized"}
      <div class="space-y-3">
        {#each normalizedFields as field (field.label)}
          <div class="flex flex-col gap-1">
            <div class="flex items-center gap-2">
              <span class="text-fg-dim text-xs tracking-wide uppercase">{field.label}</span>
              {#if field.type === "parent"}
                <button
                  class="text-blue hover:text-blue-bright text-xs"
                  onclick={() => onNavigateParent?.(field.value)}>
                  Jump to parent
                </button>
              {/if}
            </div>
            <div
              class="text-fg bg-surface-soft rounded p-2 font-mono text-sm {field.type === 'path' ? 'break-all' : ''}">
              {field.value}
            </div>
          </div>
        {/each}

        {#if event.content}
          <div class="border-surface-muted flex flex-col gap-1 border-t pt-2">
            <span class="text-fg-dim text-xs tracking-wide uppercase">Content</span>
            <div class="text-fg bg-surface-soft rounded p-3 font-mono text-sm whitespace-pre-wrap">
              {event.content}
            </div>
          </div>
        {/if}
      </div>
    {:else if activeTab === "thinking" && thinkingContent}
      <div class="space-y-4">
        <div class="flex flex-col gap-1">
          <span class="text-fg-dim text-xs tracking-wide uppercase">Thinking Content</span>
          <div class="text-fg bg-surface-soft rounded p-4 font-mono text-sm whitespace-pre-wrap">
            {thinkingContent}
          </div>
        </div>
        {#if signature}
          <div class="flex flex-col gap-1">
            <span class="text-fg-dim text-xs tracking-wide uppercase">Signature</span>
            <div class="text-fg-dim bg-surface-muted rounded p-2 font-mono text-xs break-all">{signature}</div>
          </div>
        {/if}
      </div>
    {:else if activeTab === "tools" && toolCalls.length > 0}
      <div class="space-y-4">
        {#each toolCalls as tool (tool.id)}
          <div class="border-surface-muted overflow-hidden rounded-lg border">
            <div class="bg-surface-soft flex items-center gap-2 px-3 py-2">
              <span class="i-ri-tools-line text-fg-dim"></span>
              <span class="text-fg text-sm font-medium">{tool.name}</span>
              <span class="text-fg-dim font-mono text-xs">{tool.id}</span>
            </div>
            <div class="p-3">
              <div class="text-fg-dim mb-1 text-xs tracking-wide uppercase">Input</div>
              <pre class="text-fg-dim bg-surface-muted overflow-x-auto rounded p-2 text-xs"><code
                  >{formatJson(tool.input)}</code></pre>
            </div>
          </div>
        {/each}
      </div>
    {:else if activeTab === "raw"}
      <pre class="text-fg-dim text-xs"><code>{formatJson(event.raw_payload)}</code></pre>
    {/if}
  </div>
</div>
