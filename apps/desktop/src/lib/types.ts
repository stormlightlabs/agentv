export type SessionData = {
  id: string;
  source: string;
  external_id: string;
  project: string | null;
  title: string | null;
  created_at: string;
  updated_at: string;
};

export type EventData = {
  id: string;
  session_id: string;
  kind: string;
  role: string | null;
  content: string | null;
  timestamp: string;
};

export type SearchResult = { event: EventData; rank: number; snippet: string | null };

export type SearchFacets = { source?: string; project?: string; kind?: string; since?: string };

export type ActivityStats = { day: string; event_count: number; session_count: number };

export type ErrorStats = { day: string; error_count: number; signature: string | null };

export type GroupedStats = { dimension: string; count: number; sessions?: number; earliest?: string; latest?: string };
