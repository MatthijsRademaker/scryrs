export class ApiError extends Error {
  constructor(public readonly status: number, message: string) {
    super(message);
    this.name = "ApiError";
  }
}

export interface HotspotsReport {
  schemaVersion?: string;
  command?: string;
  generatedAt?: string;
  entries: HotspotEntry[];
}

export interface HotspotEntry {
  rank: number;
  subjectKind: string;
  subject: string;
  score: number;
  counts: { eventType: Record<string, number>; outcome: Record<string, number> };
  sessionCount: number;
  firstSeen: string;
  lastSeen: string;
  evidence: { rowIds: number[] };
}

export interface SessionSummary {
  sessionId: string;
  startedAt: string;
  endedAt: string | null;
  eventCount: number;
  source: string;
}

export interface TraceEventItem {
  eventId: number;
  sessionId: string;
  eventType: string;
  timestamp: string;
  subjectKind: string | null;
  subject: string | null;
  payload: unknown;
}

export interface EventsPage {
  events: TraceEventItem[];
  nextCursor: string | null;
}

export interface SessionDetail {
  session: SessionSummary;
  events: TraceEventItem[];
}

async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url);
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText })) as { error?: string };
    throw new ApiError(response.status, body.error ?? response.statusText);
  }
  return await response.json() as T;
}

export function getHotspots(): Promise<HotspotsReport> {
  return fetchJson<HotspotsReport>("/api/hotspots");
}

export function getSessions(limit = 50): Promise<SessionSummary[]> {
  return fetchJson<SessionSummary[]>(`/api/sessions?limit=${encodeURIComponent(limit)}`);
}

export function getSession(sessionId: string): Promise<SessionDetail> {
  return fetchJson<SessionDetail>(`/api/sessions/${encodeURIComponent(sessionId)}`);
}

export function getEvents(params: { limit?: number; cursor?: string | null; sessionId?: string | null } = {}): Promise<EventsPage> {
  const query = new URLSearchParams();
  query.set("limit", String(params.limit ?? 50));
  if (params.cursor) query.set("cursor", params.cursor);
  if (params.sessionId) query.set("sessionId", params.sessionId);
  return fetchJson<EventsPage>(`/api/events?${query.toString()}`);
}
