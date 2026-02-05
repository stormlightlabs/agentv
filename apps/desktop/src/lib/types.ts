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
