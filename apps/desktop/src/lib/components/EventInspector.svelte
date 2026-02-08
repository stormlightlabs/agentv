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
</script>

<div class="flex flex-col h-full bg-bg">
  <div class="flex items-center justify-between px-4 py-3 border-b border-bg-muted">
    <div class="flex items-center gap-2">
      <span
        class="px-2 py-1 text-2xs font-semibold uppercase rounded {event.kind === 'error'
          ? 'bg-red text-bg'
          : event.kind === 'tool_call'
            ? 'bg-purple text-bg'
            : event.kind === 'tool_result'
              ? 'bg-green text-bg'
              : 'bg-bg-muted text-fg'}">
        {event.kind}
      </span>
      {#if event.role}
        <span class="text-xs text-fg-dim">{event.role}</span>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <button
        class="p-1.5 text-fg-dim hover:text-fg transition-colors"
        title="Copy ID"
        onclick={() => {
          copyToClipboard(event.id);
          onCopyId?.(event.id);
        }}>
        <span class="i-ri-clipboard-line"></span>
      </button>
      <button
        class="p-1.5 text-fg-dim hover:text-fg transition-colors"
        title="Copy Payload"
        onclick={() => {
          copyToClipboard(formatJson(payload));
          if (payload) onCopyPayload?.(payload);
        }}>
        <span class="i-ri-file-copy-line"></span>
      </button>
    </div>
  </div>

  <div class="flex border-b border-bg-muted">
    <button
      class="px-4 py-2 text-sm transition-colors {activeTab === 'normalized'
        ? 'text-fg border-b-2 border-blue'
        : 'text-fg-dim hover:text-fg'}"
      onclick={() => (activeTab = "normalized")}>
      Fields
    </button>
    {#if thinkingContent}
      <button
        class="px-4 py-2 text-sm transition-colors {activeTab === 'thinking'
          ? 'text-fg border-b-2 border-blue'
          : 'text-fg-dim hover:text-fg'}"
        onclick={() => (activeTab = "thinking")}>
        Thinking
      </button>
    {/if}
    {#if toolCalls.length > 0}
      <button
        class="px-4 py-2 text-sm transition-colors {activeTab === 'tools'
          ? 'text-fg border-b-2 border-blue'
          : 'text-fg-dim hover:text-fg'}"
        onclick={() => (activeTab = "tools")}>
        Tools ({toolCalls.length})
      </button>
    {/if}
    <button
      class="px-4 py-2 text-sm transition-colors {activeTab === 'raw'
        ? 'text-fg border-b-2 border-blue'
        : 'text-fg-dim hover:text-fg'}"
      onclick={() => (activeTab = "raw")}>
      Raw JSON
    </button>
  </div>

  <div class="flex-1 overflow-auto p-4">
    {#if activeTab === "normalized"}
      <div class="space-y-3">
        {#each normalizedFields as field}
          <div class="flex flex-col gap-1">
            <div class="flex items-center gap-2">
              <span class="text-xs text-fg-dim uppercase tracking-wide">{field.label}</span>
              {#if field.type === "parent"}
                <button
                  class="text-xs text-blue hover:text-blue-bright"
                  onclick={() => onNavigateParent?.(field.value)}>
                  Jump to parent
                </button>
              {/if}
            </div>
            <div class="text-sm text-fg font-mono bg-bg-soft p-2 rounded {field.type === 'path' ? 'break-all' : ''}">
              {field.value}
            </div>
          </div>
        {/each}

        {#if event.content}
          <div class="flex flex-col gap-1 pt-2 border-t border-bg-muted">
            <span class="text-xs text-fg-dim uppercase tracking-wide">Content</span>
            <div class="text-sm text-fg whitespace-pre-wrap font-mono bg-bg-soft p-3 rounded">
              {event.content}
            </div>
          </div>
        {/if}
      </div>
    {:else if activeTab === "thinking" && thinkingContent}
      <div class="space-y-4">
        <div class="flex flex-col gap-1">
          <span class="text-xs text-fg-dim uppercase tracking-wide">Thinking Content</span>
          <div class="text-sm text-fg whitespace-pre-wrap font-mono bg-bg-soft p-4 rounded">
            {thinkingContent}
          </div>
        </div>
        {#if signature}
          <div class="flex flex-col gap-1">
            <span class="text-xs text-fg-dim uppercase tracking-wide">Signature</span>
            <div class="text-xs text-fg-dim font-mono bg-bg-muted p-2 rounded break-all">{signature}</div>
          </div>
        {/if}
      </div>
    {:else if activeTab === "tools" && toolCalls.length > 0}
      <div class="space-y-4">
        {#each toolCalls as tool (tool.id)}
          <div class="border border-bg-muted rounded-lg overflow-hidden">
            <div class="bg-bg-soft px-3 py-2 flex items-center gap-2">
              <span class="i-ri-tools-line text-fg-dim"></span>
              <span class="text-sm font-medium text-fg">{tool.name}</span>
              <span class="text-xs text-fg-dim font-mono">{tool.id}</span>
            </div>
            <div class="p-3">
              <div class="text-xs text-fg-dim uppercase tracking-wide mb-1">Input</div>
              <pre class="text-xs text-fg-dim bg-bg-muted p-2 rounded overflow-x-auto"><code
                  >{formatJson(tool.input)}</code></pre>
            </div>
          </div>
        {/each}
      </div>
    {:else if activeTab === "raw"}
      <pre class="text-xs text-fg-dim"><code>{formatJson(event.raw_payload)}</code></pre>
    {/if}
  </div>
</div>
