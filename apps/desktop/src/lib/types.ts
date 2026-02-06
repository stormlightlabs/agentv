export type SessionData = {
  id: string;
  source: string;
  external_id: string;
  project: string | null;
  title: string | null;
  created_at: string;
  updated_at: string;
};

type TextContentBlock = { type: "text"; text: string };

type ThinkingContentBlock = { type: "thinking"; thinking: string; signature?: string };

type ToolUseContentBlock = { type: "tool_use"; id: string; name: string; input: Record<string, unknown> };

type Block = { type: string };

export type ContentBlock = TextContentBlock | ThinkingContentBlock | ToolUseContentBlock | Block;

export type ClaudeMessage = {
  role: string;
  content?: string | ContentBlock[];
  model?: string;
  usage?: Record<string, unknown>;
  stop_reason?: string;
};

export type EventPayload = {
  type?: string;
  uuid?: string;
  parentUuid?: string | null;
  timestamp?: string;
  message?: ClaudeMessage;
  content?: string;
  gitBranch?: string;
  cwd?: string;
  [key: string]: unknown;
};

export type EventData = {
  id: string;
  session_id: string;
  kind: string;
  role: string | null;
  content: string | null;
  timestamp: string;
  raw_payload: EventPayload;
};

export type SearchResult = { event: EventData; rank: number; snippet: string | null };

export type SearchFacets = { source?: string; project?: string; kind?: string; since?: string };

export type ActivityStats = { day: string; event_count: number; session_count: number };

export type ErrorStats = { day: string; error_count: number; signature: string | null };

export type GroupedStats = { dimension: string; count: number; sessions?: number; earliest?: string; latest?: string };

export type SourceHealth = {
  source: "claude" | "codex" | "opencode" | "crush";
  status: "healthy" | "degraded" | "unhealthy" | "unknown";
  path: string | null;
  message: string | null;
};

export type IngestResult = { imported: number; failed: number; total: number; source: string; duration_ms: number };

export type ToastNotification = { id: string; type: "success" | "error" | "info"; message: string; duration?: number };

export type DataTableColumn<T = Record<string, unknown>> = {
  key: keyof T | string;
  header: string;
  sortable?: boolean;
  filterable?: boolean;
  width?: string;
  render?: (row: T) => string | { text: string; className?: string };
};
