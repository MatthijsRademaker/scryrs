export class ApiError extends Error {
	constructor(
		public readonly status: number,
		message: string,
	) {
		super(message);
		this.name = "ApiError";
	}
}

export type DashboardMode = "local" | "live";

export interface HotspotsReport {
	schemaVersion?: string;
	command?: string;
	generatedAt?: string;
	repositoryPath?: string;
	repositoryId?: string;
	cursor?: string;
	entries: HotspotEntry[];
}

export interface DashboardMeta {
	mode: DashboardMode;
	repositoryPath: string;
	repositoryId?: string | null;
	proposalReadsAvailable: boolean;
	proposalReviewWritesAvailable: boolean;
}

export interface HotspotEntry {
	rank: number;
	subjectKind: string;
	subject: string;
	score: number;
	counts: {
		eventType: Record<string, number>;
		outcome: Record<string, number>;
	};
	sessionCount: number;
	firstSeen: string;
	lastSeen: string;
	evidence: { rowIds: number[] };
}

export interface HotspotSignal {
	repositoryId: string;
	subjectKind: string;
	subject: string;
	score: number;
	delta: number;
	window: string;
	threshold: number;
	evidenceRowIds: number[];
	createdAt: string;
}

export interface DashboardSignal extends HotspotSignal {
	id: number;
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

async function readError(response: Response): Promise<never> {
	const body = (await response
		.json()
		.catch(() => ({ error: response.statusText }))) as { error?: string };
	throw new ApiError(response.status, body.error ?? response.statusText);
}

async function fetchJson<T>(url: string): Promise<T> {
	const response = await fetch(url);
	if (!response.ok) {
		return readError(response);
	}
	return (await response.json()) as T;
}

async function postJson<TRequest, TResponse>(
	url: string,
	body: TRequest,
): Promise<TResponse> {
	const response = await fetch(url, {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify(body),
	});
	if (!response.ok) {
		return readError(response);
	}
	return (await response.json()) as TResponse;
}

export function fetchMeta(): Promise<DashboardMeta> {
	return fetchJson<DashboardMeta>("/api/meta");
}

export function getHotspots(): Promise<HotspotsReport> {
	return fetchJson<HotspotsReport>("/api/hotspots");
}

export function getSessions(limit = 50): Promise<SessionSummary[]> {
	return fetchJson<SessionSummary[]>(
		`/api/sessions?limit=${encodeURIComponent(limit)}`,
	);
}

export function getSession(sessionId: string): Promise<SessionDetail> {
	return fetchJson<SessionDetail>(
		`/api/sessions/${encodeURIComponent(sessionId)}`,
	);
}

export function getEvents(
	params: {
		limit?: number;
		cursor?: string | null;
		sessionId?: string | null;
	} = {},
): Promise<EventsPage> {
	const query = new URLSearchParams();
	query.set("limit", String(params.limit ?? 50));
	if (params.cursor) query.set("cursor", params.cursor);
	if (params.sessionId) query.set("sessionId", params.sessionId);
	return fetchJson<EventsPage>(`/api/events?${query.toString()}`);
}

export function getSignalStreamUrl(after: number): string {
	return `/api/signals?after=${encodeURIComponent(String(after))}`;
}

// --- Route explain DTOs ---

export interface RouteLoadTarget {
	kind: "file" | "doc_page" | "non_loadable";
	reference?: string;
}

// --- Shared evidence type ---

export interface EvidenceLink {
	sourceKind: string;
	subject: string;
	rowIds: number[];
	docRef?: string | null;
	description?: string | null;
	score?: number | null;
	metadata?: Record<string, unknown> | null;
}

// --- Route hint DTOs ---

export interface RouteHintItem {
	routeId: string;
	target: string;
	loadTarget?: RouteLoadTarget;
	label: string;
	rank: number;
	relevance?: number;
	reason: string;
	evidence?: EvidenceLink[];
}

export interface RouteHintDocument {
	schemaVersion: string;
	hints: RouteHintItem[];
}

export function getRouteHints(query: string): Promise<RouteHintDocument> {
	return fetchJson<RouteHintDocument>(
		`/api/routes/explain?query=${encodeURIComponent(query)}`,
	);
}

// --- Proposal DTOs ---

export interface ProposalListRow {
	proposalId: string;
	title: string;
	targetType: string;
	createdAt: string;
	state: "pending" | "accepted" | "rejected";
}

export interface ProposalReviewDecisionMeta {
	reviewer: string;
	outcome: string;
	decidedAt: string;
	rationale: string;
	acceptedContent?: unknown;
	targetType?: string;
}

export interface ProposalReviewSubmission {
	reviewer: string;
	rationale: string;
	decidedAt: string;
	reviewedContent?: string;
}

export interface ProposalReviewResponse {
	outcome: "accepted" | "rejected";
}

export interface ProposalDetail {
	schemaVersion: string;
	id: string;
	targetType: string;
	title: string;
	rationale: string;
	proposedContent: unknown;
	evidence: EvidenceLink[];
	createdAt: string;
	reviewDecision?: ProposalReviewDecisionMeta | null;
}

export function getProposals(): Promise<ProposalListRow[]> {
	return fetchJson<ProposalListRow[]>("/api/proposals");
}

export function getProposal(proposalId: string): Promise<ProposalDetail> {
	return fetchJson<ProposalDetail>(
		`/api/proposals/${encodeURIComponent(proposalId)}`,
	);
}

export function acceptProposalReview(
	proposalId: string,
	body: ProposalReviewSubmission,
): Promise<ProposalReviewResponse> {
	return postJson<ProposalReviewSubmission, ProposalReviewResponse>(
		`/api/proposals/${encodeURIComponent(proposalId)}/accept`,
		body,
	);
}

export function rejectProposalReview(
	proposalId: string,
	body: Omit<ProposalReviewSubmission, "reviewedContent">,
): Promise<ProposalReviewResponse> {
	return postJson<
		Omit<ProposalReviewSubmission, "reviewedContent">,
		ProposalReviewResponse
	>(`/api/proposals/${encodeURIComponent(proposalId)}/reject`, body);
}
