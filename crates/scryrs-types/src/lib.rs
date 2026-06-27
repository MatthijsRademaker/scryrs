//! Shared contracts for scryrs workspace crates.

use std::collections::HashMap;
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Version for machine-facing contracts emitted by this scaffold.
pub const SCHEMA_VERSION: &str = "0.1.0";

/// Version for the hotspot report output contract, independent of
/// `SCHEMA_VERSION` which governs trace event wire format.
pub const HOTSPOT_SCHEMA_VERSION: &str = "1.0.0";

/// Version for the live hotspot query response, independent of
/// `SCHEMA_VERSION` (trace event wire format) and
/// `HOTSPOT_SCHEMA_VERSION` (local report output).
pub const LIVE_HOTSPOT_SCHEMA_VERSION: &str = "1.0.0";

/// Version for the knowledge graph wire contract, independent of
/// `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, and
/// `LIVE_HOTSPOT_SCHEMA_VERSION`.
pub const GRAPH_SCHEMA_VERSION: &str = "1.0.0";

/// Version for the route manifest wire contract, independent of
/// `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`,
/// `LIVE_HOTSPOT_SCHEMA_VERSION`, and `GRAPH_SCHEMA_VERSION`.
pub const ROUTE_SCHEMA_VERSION: &str = "1.0.0";

/// Version for the proposal document contract, independent of
/// `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`,
/// `LIVE_HOTSPOT_SCHEMA_VERSION`, `GRAPH_SCHEMA_VERSION`, and
/// `ROUTE_SCHEMA_VERSION`.
pub const PROPOSAL_SCHEMA_VERSION: &str = "1.0.0";

/// Suite component metadata used by feature-gated crates and CLI output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
}

/// Versioned trace event envelope used by all trace producers and consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub schema_version: String,
    pub timestamp: String,
    pub session_id: String,
    pub event_type: TraceEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub payload: TraceEventPayload,
    pub outcome: Outcome,
}

impl TraceEvent {
    /// Return a hotspot subject for subject-bearing events, or `None` for
    /// lifecycle events that have no hotspot subject.
    #[must_use]
    pub fn subject(&self) -> Option<&str> {
        match &self.payload {
            TraceEventPayload::SessionStart(_) | TraceEventPayload::SessionEnd(_) => None,
            TraceEventPayload::FileOpened(p) => Some(p.path.as_str()),
            TraceEventPayload::SearchRun(p) => Some(p.query.as_str()),
            TraceEventPayload::SymbolInspected(p) => Some(p.name.as_str()),
            TraceEventPayload::CommandExecuted(p) => Some(p.command.as_str()),
            TraceEventPayload::DocRetrieved(p) => Some(p.doc_ref.as_str()),
            TraceEventPayload::EditMade(p) => Some(p.target.as_str()),
            TraceEventPayload::FailedLookup(p) => Some(p.subject.as_str()),
        }
    }

    /// Return a short category tag for subject-bearing events, or `None`
    /// for lifecycle events. This is the `subject_kind` column used for
    /// indexed subject lookup in the datastore.
    #[must_use]
    pub fn subject_kind(&self) -> Option<&'static str> {
        match &self.payload {
            TraceEventPayload::SessionStart(_) | TraceEventPayload::SessionEnd(_) => None,
            TraceEventPayload::FileOpened(_) | TraceEventPayload::EditMade(_) => Some("file"),
            TraceEventPayload::SearchRun(_) => Some("search"),
            TraceEventPayload::SymbolInspected(_) | TraceEventPayload::FailedLookup(_) => {
                Some("symbol")
            }
            TraceEventPayload::CommandExecuted(_) => Some("command"),
            TraceEventPayload::DocRetrieved(_) => Some("document"),
        }
    }

    /// Extract the failure reason string for `Outcome::Failure` events,
    /// or `None` for success outcomes.
    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        match &self.outcome {
            Outcome::Success => None,
            Outcome::Failure { reason } => reason.as_deref(),
        }
    }

    /// Validate semantic invariants for an event that has passed
    /// structural deserialization. Returns `Ok(())` when both
    /// `schema_version` equals `SCHEMA_VERSION` and `event_type`
    /// matches the concrete `payload.type` tag.
    ///
    /// The caller should treat an `Err(reason)` as a rejection.
    #[must_use = "callers must check semantic invariants; discarded Result hides invalid events"]
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "schema_version mismatch: got '{}', expected '{}'",
                self.schema_version, SCHEMA_VERSION,
            ));
        }
        let expected_type = self.event_type.payload_type_str();
        let actual_type = self.payload.payload_type_str();
        if expected_type != actual_type {
            return Err(format!(
                "event_type/payload.type mismatch: event_type='{}', payload.type='{}'",
                expected_type, actual_type,
            ));
        }
        Ok(())
    }
}

/// Kind of trace event, mirroring the payload variant in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceEventType {
    SessionStart,
    SessionEnd,
    FileOpened,
    SearchRun,
    SymbolInspected,
    CommandExecuted,
    DocRetrieved,
    EditMade,
    FailedLookup,
}

impl TraceEventType {
    /// Return the expected `payload.type` string tag for this event type.
    #[must_use]
    pub fn payload_type_str(self) -> &'static str {
        match self {
            TraceEventType::SessionStart => "SessionStart",
            TraceEventType::SessionEnd => "SessionEnd",
            TraceEventType::FileOpened => "FileOpened",
            TraceEventType::SearchRun => "SearchRun",
            TraceEventType::SymbolInspected => "SymbolInspected",
            TraceEventType::CommandExecuted => "CommandExecuted",
            TraceEventType::DocRetrieved => "DocRetrieved",
            TraceEventType::EditMade => "EditMade",
            TraceEventType::FailedLookup => "FailedLookup",
        }
    }
}

/// Success or failure outcome carried on every trace event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum Outcome {
    Success,
    Failure {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

/// Payload families, one per activity kind. Self-describing on the wire via
/// the `type` tag so consumers can identify the concrete shape from JSON alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TraceEventPayload {
    SessionStart(SessionStartPayload),
    SessionEnd(SessionEndPayload),
    FileOpened(FileOpenedPayload),
    SearchRun(SearchRunPayload),
    SymbolInspected(SymbolInspectedPayload),
    CommandExecuted(CommandExecutedPayload),
    DocRetrieved(DocRetrievedPayload),
    EditMade(EditMadePayload),
    FailedLookup(FailedLookupPayload),
}

impl TraceEventPayload {
    /// Return the `type` tag string for this payload variant.
    #[must_use]
    pub fn payload_type_str(&self) -> &'static str {
        match self {
            TraceEventPayload::SessionStart(_) => "SessionStart",
            TraceEventPayload::SessionEnd(_) => "SessionEnd",
            TraceEventPayload::FileOpened(_) => "FileOpened",
            TraceEventPayload::SearchRun(_) => "SearchRun",
            TraceEventPayload::SymbolInspected(_) => "SymbolInspected",
            TraceEventPayload::CommandExecuted(_) => "CommandExecuted",
            TraceEventPayload::DocRetrieved(_) => "DocRetrieved",
            TraceEventPayload::EditMade(_) => "EditMade",
            TraceEventPayload::FailedLookup(_) => "FailedLookup",
        }
    }
}

// --- Per-family payload types ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStartPayload;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEndPayload;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOpenedPayload {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRunPayload {
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolInspectedPayload {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandExecutedPayload {
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocRetrievedPayload {
    pub doc_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditMadePayload {
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedLookupPayload {
    pub subject: String,
}

// --- Adjacent types (unchanged from scaffold) ---

/// Ranked hotspot entry carrying full evidence from deterministic analysis.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotspotEntry {
    pub rank: u32,
    pub subjectKind: String,
    pub subject: String,
    pub score: u32,
    pub counts: HotspotCounts,
    pub sessionCount: u32,
    pub firstSeen: String,
    pub lastSeen: String,
    pub evidence: HotspotEvidence,
}

/// Per-event-type and per-outcome breakdown counts for a hotspot entry.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotspotCounts {
    pub eventType: HashMap<String, u32>,
    pub outcome: HashMap<String, u32>,
}

/// Ordered SQLite row ID references for all contributing events.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotspotEvidence {
    pub rowIds: Vec<u64>,
}

/// Top-level hotspot report envelope emitted to stdout and `.scryrs/hotspots.json`.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HotspotsReport {
    pub schemaVersion: String,
    pub command: String,
    pub repositoryPath: String,
    pub storePath: String,
    pub runMetadata: RunMetadata,
    pub generatedAt: String,
    pub entries: Vec<HotspotEntry>,
}

/// Deterministic metadata derived from the SQLite store state.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunMetadata {
    pub storeSchemaVersion: i64,
    pub analyzedEventCount: u64,
    pub analyzedSubjectCount: u64,
    pub firstEventId: u64,
    pub lastEventId: u64,
}

// --- Live hotspot accumulator and signal types ---

/// Deterministic signal persisted when a cumulative hotspot score crosses
/// the configured threshold. Each signal is append-only and stored
/// separately from accumulator rows.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotspotSignal {
    /// Repository that produced the event.
    pub repositoryId: String,
    /// Subject kind tag ("file", "search", "symbol", "command", "document").
    pub subjectKind: String,
    /// Concrete subject string.
    pub subject: String,
    /// Cumulative score at the time of the crossing.
    pub score: u32,
    /// Score delta contributed by the triggering event.
    pub delta: u32,
    /// Window model tag — always `"cumulative"` for this foundation.
    pub window: String,
    /// Configured threshold that was crossed.
    pub threshold: u32,
    /// Ordered server_trace_events row IDs contributing to this signal.
    pub evidenceRowIds: Vec<u64>,
    /// RFC 3339 timestamp when the signal was created.
    pub createdAt: String,
}

// --- Live hotspot server contract types (Phase 4) ---

/// Versioned batch wrapper for trace events submitted to the live hotspot server.
/// Carries submission-context identity fields and an array of per-event items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerIngestEnvelope {
    pub envelope_version: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub agent_id: String,
    pub events: Vec<EnvelopeEvent>,
}

/// Per-event item within a `ServerIngestEnvelope`, pairing producer-scoped
/// identity and timing metadata with the inner `TraceEvent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeEvent {
    pub producer_event_id: String,
    pub client_timestamp: String,
    pub event: TraceEvent,
}

/// Acknowledgment status for a single event within a batch ingest response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAckStatus {
    /// First submission of this event — a new record was created.
    Accepted,
    /// Duplicate submission — already processed, no new record created.
    Idempotent,
    /// Event failed server-side validation — see `EventAck.error_reason`.
    Rejected,
}

/// Per-event acknowledgment returned in `BatchIngestResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventAck {
    /// Zeroth-indexed position of this item in the request `events` array.
    pub index: usize,
    /// Producer-scoped event identifier; `None` only when the request item
    /// could not supply one (malformed per-item decode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer_event_id: Option<String>,
    pub status: EventAckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
    pub received_at: String,
}

/// JSON acknowledgment returned by `POST /v1/trace-events/batch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchIngestResponse {
    /// Count of events accepted (first-writer-wins) in this batch.
    pub accepted_count: u64,
    /// Count of duplicate (idempotent) events in this batch.
    pub duplicate_count: u64,
    /// Count of rejected events in this batch.
    pub rejected_count: u64,
    /// Count of accepted events in this batch (excluding idempotent).
    pub received_count: u64,
    pub events: Vec<EventAck>,
    pub received_at: String,
}

/// Live hotspot query response envelope for `GET /v1/repositories/{id}/hotspots`.
/// Separate from the local-only `HotspotsReport` — carries no filesystem-path fields.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveHotspotsResponse {
    pub schemaVersion: String,
    pub repositoryId: String,
    pub cursor: String,
    pub generatedAt: String,
    pub entries: Vec<HotspotEntry>,
}

// --- Knowledge graph wire-contract types ---

/// Closed set of evidence provenance kinds carried by `EvidenceLink`.
/// Serialized as snake_case strings on the wire.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceKind {
    /// Link to a hotspot subject identity (no trace row granularity).
    HotspotSubject,
    /// Link to one or more local `trace_events.id` rows.
    LocalTraceRow,
    /// Link to one or more live `server_trace_events.id` rows.
    ServerTraceRow,
    /// Link to a documentation reference.
    DocReference,
    /// Link to a recorded evidence descriptor.
    RecordedEvidence,
}

/// Flat evidence link attaching provenance to a graph node or edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceLink {
    /// Provenance source category.
    pub source_kind: EvidenceSourceKind,
    /// Subject that this evidence applies to (e.g. hotspot subject).
    pub subject: String,
    /// Ordered row IDs from the source data store, empty when not applicable.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub row_ids: Vec<u64>,
    /// Documentation reference for `DocReference` links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_ref: Option<String>,
    /// Recorded evidence descriptor for `RecordedEvidence` links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional score snapshot; not part of stable provenance identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<u32>,
    /// Additive namespaced metadata extension map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl Eq for EvidenceLink {}

/// Top-level metadata carried on a knowledge graph document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphMetadata {
    /// Repository identifier, present for server-owned graphs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository_id: Option<String>,
    /// Additive namespaced metadata extension map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Route-ready graph node carrying identity, filtering, and evidence fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    /// Stable unique node identifier.
    pub id: String,
    /// Human-readable label for display and filtering.
    pub label: String,
    /// Optional longer description or summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// String-backed node kind for additive adapter evolution.
    pub kind: String,
    /// Searchable and filterable tags, deterministically sorted.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    /// Alternative names for this node, deterministically sorted.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub aliases: Vec<String>,
    /// Evidence provenance links for this node.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub evidence_links: Vec<EvidenceLink>,
    /// Additive namespaced metadata extension map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Directed graph edge connecting two nodes with evidence provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    /// Stable unique edge identifier.
    pub id: String,
    /// Source node identified by its `GraphNode.id`.
    pub source_node_id: String,
    /// Target node identified by its `GraphNode.id`.
    pub target_node_id: String,
    /// String-backed relationship kind for additive adapter evolution.
    pub relationship: String,
    /// Optional human-readable label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Searchable and filterable tags, deterministically sorted.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    /// Evidence provenance links for this edge.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub evidence_links: Vec<EvidenceLink>,
    /// Additive namespaced metadata extension map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Versioned knowledge graph document — the top-level wire contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeGraphDocument {
    /// Schema version, always equal to `GRAPH_SCHEMA_VERSION`.
    pub schema_version: String,
    /// Top-level graph metadata including optional repository context.
    pub metadata: GraphMetadata,
    /// Deterministically ordered graph nodes.
    pub nodes: Vec<GraphNode>,
    /// Deterministically ordered directed graph edges.
    pub edges: Vec<GraphEdge>,
}

// --- Proposal document contract types ---

/// Closed set of proposal target types — each proposal must target exactly
/// one kind of reviewable knowledge artifact. Serialized as snake_case strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalTargetType {
    DocsNote,
    Adr,
    Skill,
    DebuggingPlaybook,
    MemoryPatch,
    SemanticGraphGrouping,
}

/// Target-type-specific proposed content carried in a `ProposalDocument`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProposedContent {
    /// Non-empty markdown text for `docs_note`, `adr`, `skill`, and
    /// `debugging_playbook` targets. Must be tried first: a JSON string
    /// is also a JSON value, so this must precede `MemoryPatch`.
    Markdown(String),
    /// Structured grouping object for `semantic_graph_grouping` targets.
    /// Must precede `MemoryPatch` so concrete grouping objects are not
    /// deserialized as opaque `serde_json::Value`.
    SemanticGraphGrouping(SemanticGraphGrouping),
    /// Structured JSON object for `memory_patch` targets.
    /// Must be last: any JSON value can match, so it acts as a fallback
    /// for everything that isn't a markdown string or a grouping object.
    MemoryPatch(serde_json::Value),
}

impl ProposedContent {
    /// Canonical serialized JSON representation for deterministic id computation.
    /// Returns a stable, whitespace-free JSON string.
    pub fn canonical_json(&self) -> Result<String, serde_json::Error> {
        // Use compact serialization for deterministic content addressing.
        serde_json::to_string(self)
    }
}

/// Structured content payload for `semantic_graph_grouping` proposals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticGraphGrouping {
    /// Exact source graph node IDs that this grouping aggregates (non-empty).
    pub source_node_ids: Vec<String>,
    /// Proposed identifier for the parent group node.
    pub target_group_node_id: String,
    /// Human-readable label for the parent group node.
    pub target_group_label: String,
}

/// Versioned proposal document — the review-only inbox artifact.
///
/// Proposal documents live under `.scryrs/proposals/` as individual JSON
/// files. Each filename stem equals the deterministic SHA-256 `id` derived
/// from `targetType` plus the canonical serialized `proposedContent`.
/// Proposal files are review artifacts only and never directly mutate
/// published docs, ADRs, skills, playbooks, memory truth, `.scryrs/graph.json`,
/// or `.scryrs/routes.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDocument {
    /// Schema version, always equal to `PROPOSAL_SCHEMA_VERSION`.
    pub schema_version: String,
    /// Deterministic SHA-256 content address derived from `targetType` plus
    /// the canonical serialized `proposedContent`.
    pub id: String,
    /// One of the six defined proposal target kinds.
    pub target_type: ProposalTargetType,
    /// Short human-readable title for the proposal.
    pub title: String,
    /// Non-empty explanation of why this proposal should be considered.
    pub rationale: String,
    /// Target-type-specific proposed content — must be non-empty.
    pub proposed_content: ProposedContent,
    /// Evidence provenance links — must be non-empty.
    pub evidence: Vec<EvidenceLink>,
    /// RFC 3339 creation timestamp.
    pub created_at: String,
}

impl ProposalDocument {
    /// Compute the deterministic proposal `id` from its type and content.
    ///
    /// The id is a hex-encoded SHA-256 digest of `targetType` (snake_case
    /// string) concatenated with a colon separator and the canonical
    /// serialized `proposedContent`.
    ///
    /// This produces stable fingerprints: two proposals with the same target
    /// type and same proposed content always receive the same id.
    pub fn compute_id(
        target_type: &ProposalTargetType,
        proposed_content: &ProposedContent,
    ) -> Result<String, serde_json::Error> {
        let type_str = serde_json::to_string(target_type)?;
        // Strip surrounding quotes from the serialized enum string (e.g. `"docs_note"` -> `docs_note`).
        let type_str = type_str.trim_matches('"');
        let content_json = proposed_content.canonical_json()?;
        let input = format!("{type_str}:{content_json}");
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for byte in &result {
            write!(&mut hex, "{byte:02x}").map_err(|e| serde::ser::Error::custom(e.to_string()))?;
        }
        Ok(hex)
    }

    /// Validate semantic invariants for a proposal that has passed
    /// structural deserialization. Returns `Ok(())` when `schema_version`
    /// equals `PROPOSAL_SCHEMA_VERSION`, `title` is non-empty,
    /// `rationale` is non-empty, `evidence` is non-empty, and
    /// `proposed_content` satisfies target-type specific rules.
    #[must_use = "callers must check semantic invariants; discarded Result hides invalid proposals"]
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != PROPOSAL_SCHEMA_VERSION {
            return Err(format!(
                "proposal schema_version mismatch: got '{}', expected '{}'",
                self.schema_version, PROPOSAL_SCHEMA_VERSION,
            ));
        }
        if self.title.trim().is_empty() {
            return Err("proposal title must be non-empty".into());
        }
        if self.rationale.trim().is_empty() {
            return Err("proposal rationale must be non-empty".into());
        }
        if self.evidence.is_empty() {
            return Err("proposal evidence must be non-empty".into());
        }

        match &self.proposed_content {
            ProposedContent::Markdown(text) => {
                if text.trim().is_empty() {
                    return Err("proposal proposed_content markdown must be non-empty".into());
                }
            }
            ProposedContent::MemoryPatch(value) => {
                if value.is_null() {
                    return Err("proposal proposed_content memory_patch must be non-null".into());
                }
            }
            ProposedContent::SemanticGraphGrouping(grouping) => {
                if grouping.source_node_ids.is_empty() {
                    return Err(
                        "semantic graph grouping proposal must have non-empty sourceNodeIds".into(),
                    );
                }
            }
        }

        Ok(())
    }

    /// Return the deterministic inbox filename for this proposal.
    /// The file should be written as `.scryrs/proposals/{filename}.json`.
    #[must_use]
    pub fn inbox_filename(&self) -> String {
        format!("{}.json", self.id)
    }
}

/// Optional grouping for a route entry, derived from an explicit
/// `contains` edge from a parent group node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteGrouping {
    /// The parent node's `id`.
    pub group_id: String,
    /// The parent node's `label`.
    pub group_label: String,
}

/// A single route entry with identity, target, and evidence backlinks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteEntry {
    /// Stable unique node identifier (matches source graph node `id`).
    pub id: String,
    /// Source graph node kind (e.g. `file`, `search`, `doc_page`).
    pub subject_kind: String,
    /// Raw subject value from the source graph node.
    pub subject: String,
    /// Human-readable label for display and filtering.
    pub label: String,
    /// Normalized load target (equals source graph node `id`).
    pub target: String,
    /// String-backed kind for additive adapter evolution.
    pub kind: String,
    /// Evidence provenance links for this route.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub evidence_links: Vec<EvidenceLink>,
    /// Optional grouping derived from an explicit `contains` edge
    /// from a parent group node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping: Option<RouteGrouping>,
    /// Additive namespaced metadata extension map.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Versioned route manifest document — the top-level wire contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteManifestDocument {
    /// Schema version, always equal to `ROUTE_SCHEMA_VERSION`.
    pub schema_version: String,
    /// Top-level metadata carrying optional repository context.
    pub metadata: GraphMetadata,
    /// Deterministically ordered route entries.
    pub routes: Vec<RouteEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serialize_json<T: serde::Serialize>(value: &T) -> String {
        match serde_json::to_string(value) {
            Ok(json) => json,
            Err(error) => panic!("serialize: {error}"),
        }
    }

    fn deserialize_json<T: serde::de::DeserializeOwned>(json: &str) -> T {
        match serde_json::from_str(json) {
            Ok(value) => value,
            Err(error) => panic!("deserialize: {error}"),
        }
    }

    #[test]
    fn schema_version_starts_at_initial_scaffold_version() {
        assert_eq!(SCHEMA_VERSION, "0.1.0");
    }

    // --- Subject extraction ---

    #[test]
    fn lifecycle_events_return_no_subject() {
        let start = TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T00:00:00Z".into(),
            session_id: "s1".into(),
            event_type: TraceEventType::SessionStart,
            tool_name: None,
            payload: TraceEventPayload::SessionStart(SessionStartPayload),
            outcome: Outcome::Success,
        };
        let end = TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T00:00:00Z".into(),
            session_id: "s1".into(),
            event_type: TraceEventType::SessionEnd,
            tool_name: None,
            payload: TraceEventPayload::SessionEnd(SessionEndPayload),
            outcome: Outcome::Success,
        };
        assert!(start.subject().is_none());
        assert!(end.subject().is_none());
    }

    #[test]
    fn subject_bearing_events_return_the_correct_subject() {
        let events: Vec<TraceEvent> = vec![
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::FileOpened,
                tool_name: Some("read".into()),
                payload: TraceEventPayload::FileOpened(FileOpenedPayload {
                    path: "src/a.rs".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::SearchRun,
                tool_name: Some("search".into()),
                payload: TraceEventPayload::SearchRun(SearchRunPayload {
                    query: "routing".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::SymbolInspected,
                tool_name: Some("inspect".into()),
                payload: TraceEventPayload::SymbolInspected(SymbolInspectedPayload {
                    name: "MyStruct".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::CommandExecuted,
                tool_name: Some("bash".into()),
                payload: TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                    command: "cargo build".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::DocRetrieved,
                tool_name: Some("read".into()),
                payload: TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                    doc_ref: "api/foo.md".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::EditMade,
                tool_name: Some("edit".into()),
                payload: TraceEventPayload::EditMade(EditMadePayload {
                    target: "src/b.rs".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::FailedLookup,
                tool_name: Some("search".into()),
                payload: TraceEventPayload::FailedLookup(FailedLookupPayload {
                    subject: "missing_symbol".into(),
                }),
                outcome: Outcome::Failure {
                    reason: Some("not found".into()),
                },
            },
        ];

        let subjects: Vec<&str> = events.iter().filter_map(|e| e.subject()).collect();
        assert_eq!(
            subjects,
            vec![
                "src/a.rs",
                "routing",
                "MyStruct",
                "cargo build",
                "api/foo.md",
                "src/b.rs",
                "missing_symbol",
            ]
        );
    }

    // --- Serde round-trip tests (Task 3.1, 3.2, 3.3) ---

    fn round_trip(event: &TraceEvent) {
        let json = match serde_json::to_string_pretty(event) {
            Ok(v) => v,
            Err(e) => panic!("serialization failed: {e}"),
        };
        // Every event must carry the schema version in its JSON.
        assert!(
            json.contains(SCHEMA_VERSION),
            "serialized JSON must contain schema version '{}'",
            SCHEMA_VERSION
        );
        let reconstructed: TraceEvent = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("deserialization failed: {e}"),
        };
        assert_eq!(
            &reconstructed, event,
            "round-tripped event must equal original"
        );
    }

    fn make_event(
        event_type: TraceEventType,
        tool_name: Option<&str>,
        payload: TraceEventPayload,
        outcome: Outcome,
    ) -> TraceEvent {
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T12:00:00Z".into(),
            session_id: "test-session-1".into(),
            event_type,
            tool_name: tool_name.map(Into::into),
            payload,
            outcome,
        }
    }

    #[test]
    fn session_start_round_trips() {
        round_trip(&make_event(
            TraceEventType::SessionStart,
            None,
            TraceEventPayload::SessionStart(SessionStartPayload),
            Outcome::Success,
        ));
    }

    #[test]
    fn session_end_round_trips() {
        round_trip(&make_event(
            TraceEventType::SessionEnd,
            None,
            TraceEventPayload::SessionEnd(SessionEndPayload),
            Outcome::Success,
        ));
    }

    #[test]
    fn file_opened_round_trips() {
        round_trip(&make_event(
            TraceEventType::FileOpened,
            Some("read"),
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/main.rs".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn search_run_round_trips() {
        round_trip(&make_event(
            TraceEventType::SearchRun,
            Some("search"),
            TraceEventPayload::SearchRun(SearchRunPayload {
                query: "error handling".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn symbol_inspected_round_trips() {
        round_trip(&make_event(
            TraceEventType::SymbolInspected,
            Some("inspect"),
            TraceEventPayload::SymbolInspected(SymbolInspectedPayload {
                name: "Dispatcher".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn command_executed_round_trips() {
        round_trip(&make_event(
            TraceEventType::CommandExecuted,
            Some("bash"),
            TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                command: "cargo test".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn doc_retrieved_round_trips() {
        round_trip(&make_event(
            TraceEventType::DocRetrieved,
            Some("read"),
            TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                doc_ref: "docs/api.md".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn edit_made_round_trips() {
        round_trip(&make_event(
            TraceEventType::EditMade,
            Some("edit"),
            TraceEventPayload::EditMade(EditMadePayload {
                target: "src/lib.rs".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn failed_lookup_with_failure_outcome_round_trips() {
        round_trip(&make_event(
            TraceEventType::FailedLookup,
            Some("search"),
            TraceEventPayload::FailedLookup(FailedLookupPayload {
                subject: "nonexistent_fn".into(),
            }),
            Outcome::Failure {
                reason: Some("symbol not found".into()),
            },
        ));
    }

    #[test]
    fn schema_version_present_in_every_serialized_event() {
        // Explicitly checks every event type carries schema_version in JSON.
        let all_events = vec![
            (
                TraceEventType::SessionStart,
                None,
                TraceEventPayload::SessionStart(SessionStartPayload),
                Outcome::Success,
            ),
            (
                TraceEventType::SessionEnd,
                None,
                TraceEventPayload::SessionEnd(SessionEndPayload),
                Outcome::Success,
            ),
            (
                TraceEventType::FileOpened,
                Some("read"),
                TraceEventPayload::FileOpened(FileOpenedPayload {
                    path: "a.rs".into(),
                }),
                Outcome::Success,
            ),
            (
                TraceEventType::SearchRun,
                Some("search"),
                TraceEventPayload::SearchRun(SearchRunPayload { query: "q".into() }),
                Outcome::Success,
            ),
            (
                TraceEventType::SymbolInspected,
                Some("inspect"),
                TraceEventPayload::SymbolInspected(SymbolInspectedPayload { name: "N".into() }),
                Outcome::Success,
            ),
            (
                TraceEventType::CommandExecuted,
                Some("bash"),
                TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                    command: "c".into(),
                }),
                Outcome::Success,
            ),
            (
                TraceEventType::DocRetrieved,
                Some("read"),
                TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                    doc_ref: "d".into(),
                }),
                Outcome::Success,
            ),
            (
                TraceEventType::EditMade,
                Some("edit"),
                TraceEventPayload::EditMade(EditMadePayload { target: "t".into() }),
                Outcome::Success,
            ),
            (
                TraceEventType::FailedLookup,
                Some("search"),
                TraceEventPayload::FailedLookup(FailedLookupPayload {
                    subject: "s".into(),
                }),
                Outcome::Failure {
                    reason: Some("r".into()),
                },
            ),
        ];

        for (event_type, tool_name, payload, outcome) in all_events {
            let event = make_event(event_type, tool_name, payload, outcome);
            let json = match serde_json::to_string(&event) {
                Ok(v) => v,
                Err(e) => panic!("serialize: {e}"),
            };
            assert!(
                json.contains(SCHEMA_VERSION),
                "event type {event_type:?} must carry schema_version in JSON"
            );
        }
    }

    #[test]
    fn payloads_are_self_describing_via_type_tag() {
        // Every serialized payload must include a "type" field that identifies
        // the concrete payload family from JSON alone.
        let event = make_event(
            TraceEventType::FileOpened,
            Some("read"),
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "x.rs".into(),
            }),
            Outcome::Success,
        );
        let json = match serde_json::to_string(&event) {
            Ok(v) => v,
            Err(e) => panic!("serialize: {e}"),
        };
        assert!(
            json.contains("\"type\":\"FileOpened\""),
            "payload must include self-describing type tag"
        );
    }

    // --- TraceEvent::validate() semantic invariant tests ---

    #[test]
    fn validate_accepts_semantically_correct_event() {
        let event = make_event(
            TraceEventType::FileOpened,
            Some("read"),
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "x.rs".into(),
            }),
            Outcome::Success,
        );
        assert!(event.validate().is_ok());
    }

    #[test]
    fn validate_rejects_wrong_schema_version() {
        let mut event = make_event(
            TraceEventType::DocRetrieved,
            Some("read"),
            TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                doc_ref: "d.md".into(),
            }),
            Outcome::Success,
        );
        event.schema_version = "0.9.9".into();
        let validate_result = event.validate();
        assert!(
            validate_result.is_err(),
            "version mismatch should be rejected"
        );
        let err = match validate_result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("schema_version mismatch"));
        assert!(err.contains("0.9.9"));
        assert!(err.contains(SCHEMA_VERSION));
    }

    #[test]
    fn validate_rejects_event_type_payload_mismatch() {
        let mut event = make_event(
            TraceEventType::FileOpened,
            Some("read"),
            // payload type is FileOpened, but we'll swap event_type
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "x.rs".into(),
            }),
            Outcome::Success,
        );
        event.event_type = TraceEventType::DocRetrieved;
        let validate_result = event.validate();
        assert!(validate_result.is_err(), "type mismatch should be rejected");
        let err = match validate_result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("event_type/payload.type mismatch"));
        assert!(err.contains("DocRetrieved"));
        assert!(err.contains("FileOpened"));
    }

    #[test]
    fn validate_all_lifecycle_events_are_accepted() {
        for (event_type, payload) in [
            (
                TraceEventType::SessionStart,
                TraceEventPayload::SessionStart(SessionStartPayload),
            ),
            (
                TraceEventType::SessionEnd,
                TraceEventPayload::SessionEnd(SessionEndPayload),
            ),
        ] {
            let event = make_event(event_type, None, payload, Outcome::Success);
            assert!(
                event.validate().is_ok(),
                "validate failed for {event_type:?}"
            );
        }
    }

    #[test]
    fn no_harness_specific_fields_in_json() {
        // Verify the wire format does not contain harness-specific identifiers.
        let event = make_event(
            TraceEventType::CommandExecuted,
            Some("bash"),
            TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                command: "cargo build".into(),
            }),
            Outcome::Success,
        );
        let json = match serde_json::to_string(&event) {
            Ok(v) => v,
            Err(e) => panic!("serialize: {e}"),
        };
        // Harness-specific terms that must not appear.
        for forbidden in &["harness", "stdout", "stderr", "diff", "body", "content"] {
            assert!(
                !json.contains(forbidden),
                "JSON must not contain harness-specific field: '{forbidden}'"
            );
        }
    }

    // --- Hotspot types (Hotspot Foundation 02) ---

    #[test]
    fn hotspot_schema_version_is_independent() {
        assert_eq!(HOTSPOT_SCHEMA_VERSION, "1.0.0");
        assert_ne!(HOTSPOT_SCHEMA_VERSION, SCHEMA_VERSION);
    }

    #[test]
    fn hotspot_entry_serialization_round_trip() {
        let mut event_type_counts = HashMap::new();
        event_type_counts.insert("FileOpened".to_string(), 3u32);
        event_type_counts.insert("EditMade".to_string(), 2u32);

        let mut outcome_counts = HashMap::new();
        outcome_counts.insert("success".to_string(), 4u32);
        outcome_counts.insert("failure".to_string(), 1u32);

        let entry = HotspotEntry {
            rank: 1,
            subjectKind: "file".to_string(),
            subject: "src/main.rs".to_string(),
            score: 11,
            counts: HotspotCounts {
                eventType: event_type_counts,
                outcome: outcome_counts,
            },
            sessionCount: 2,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T12:00:00Z".to_string(),
            evidence: HotspotEvidence {
                rowIds: vec![3, 7, 12, 45, 67],
            },
        };

        let json = match serde_json::to_string(&entry) {
            Ok(v) => v,
            Err(e) => panic!("serialize HotspotEntry: {e}"),
        };
        let parsed: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("deserialize HotspotEntry JSON: {e}"),
        };

        assert_eq!(parsed["rank"], 1);
        assert_eq!(parsed["subjectKind"], "file");
        assert_eq!(parsed["subject"], "src/main.rs");
        assert_eq!(parsed["score"], 11);
        assert_eq!(parsed["sessionCount"], 2);
        assert_eq!(parsed["firstSeen"], "2026-06-21T09:00:00Z");
        assert_eq!(parsed["lastSeen"], "2026-06-21T12:00:00Z");
        assert_eq!(parsed["counts"]["eventType"]["FileOpened"], 3);
        assert_eq!(parsed["counts"]["eventType"]["EditMade"], 2);
        assert_eq!(parsed["counts"]["outcome"]["success"], 4);
        assert_eq!(parsed["counts"]["outcome"]["failure"], 1);
        assert_eq!(
            parsed["evidence"]["rowIds"]
                .as_array()
                .unwrap_or_else(|| panic!("rowIds not an array"))
                .len(),
            5
        );
    }

    #[test]
    fn hotspots_report_envelope_serialization_round_trip() {
        let mut event_type_counts = HashMap::new();
        event_type_counts.insert("SearchRun".to_string(), 1u32);
        let mut outcome_counts = HashMap::new();
        outcome_counts.insert("success".to_string(), 1u32);

        let entry = HotspotEntry {
            rank: 1,
            subjectKind: "search".to_string(),
            subject: "routing".to_string(),
            score: 2,
            counts: HotspotCounts {
                eventType: event_type_counts,
                outcome: outcome_counts,
            },
            sessionCount: 1,
            firstSeen: "2026-06-21T10:00:00Z".to_string(),
            lastSeen: "2026-06-21T10:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: vec![5] },
        };

        let report = HotspotsReport {
            schemaVersion: HOTSPOT_SCHEMA_VERSION.into(),
            command: "hotspots".into(),
            repositoryPath: "/abs/path".into(),
            storePath: "/abs/path/.scryrs/scryrs.db".into(),
            runMetadata: RunMetadata {
                storeSchemaVersion: 1,
                analyzedEventCount: 1,
                analyzedSubjectCount: 1,
                firstEventId: 1,
                lastEventId: 1,
            },
            generatedAt: "2026-06-21T12:00:00Z".into(),
            entries: vec![entry],
        };

        let json = match serde_json::to_string(&report) {
            Ok(v) => v,
            Err(e) => panic!("serialize HotspotsReport: {e}"),
        };
        let parsed: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("deserialize HotspotsReport JSON: {e}"),
        };

        assert_eq!(parsed["schemaVersion"], "1.0.0");
        assert_eq!(parsed["command"], "hotspots");
        assert_eq!(parsed["repositoryPath"], "/abs/path");
        assert_eq!(parsed["storePath"], "/abs/path/.scryrs/scryrs.db");
        assert_eq!(parsed["runMetadata"]["storeSchemaVersion"], 1);
        assert_eq!(parsed["runMetadata"]["analyzedEventCount"], 1);
        assert_eq!(parsed["runMetadata"]["analyzedSubjectCount"], 1);
        assert_eq!(parsed["generatedAt"], "2026-06-21T12:00:00Z");
        assert_eq!(
            parsed["entries"]
                .as_array()
                .unwrap_or_else(|| panic!("entries not an array"))
                .len(),
            1
        );
    }

    #[test]
    fn empty_entries_hotspots_report_serializes_correctly() {
        let report = HotspotsReport {
            schemaVersion: HOTSPOT_SCHEMA_VERSION.into(),
            command: "hotspots".into(),
            repositoryPath: "/abs/path".into(),
            storePath: "/abs/path/.scryrs/scryrs.db".into(),
            runMetadata: RunMetadata {
                storeSchemaVersion: 1,
                analyzedEventCount: 0,
                analyzedSubjectCount: 0,
                firstEventId: 0,
                lastEventId: 0,
            },
            generatedAt: "2026-06-21T12:00:00Z".into(),
            entries: vec![],
        };

        let json = match serde_json::to_string(&report) {
            Ok(v) => v,
            Err(e) => panic!("serialize empty report: {e}"),
        };
        let parsed: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("deserialize empty report: {e}"),
        };
        assert_eq!(
            parsed["entries"]
                .as_array()
                .unwrap_or_else(|| panic!("entries not an array"))
                .len(),
            0
        );
        assert_eq!(parsed["runMetadata"]["analyzedEventCount"], 0);
    }

    #[test]
    fn hotspot_entry_round_trip_via_value() {
        // Full serialization round-trip through serde_json::Value.
        let original = HotspotEntry {
            rank: 2,
            subjectKind: "command".to_string(),
            subject: "cargo build".to_string(),
            score: 1,
            counts: HotspotCounts {
                eventType: {
                    let mut m = HashMap::new();
                    m.insert("CommandExecuted".to_string(), 1u32);
                    m
                },
                outcome: {
                    let mut m = HashMap::new();
                    m.insert("success".to_string(), 1u32);
                    m
                },
            },
            sessionCount: 1,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T09:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: vec![42] },
        };

        let json = match serde_json::to_string(&original) {
            Ok(v) => v,
            Err(e) => panic!("serialize: {e}"),
        };
        let deserialized: HotspotEntry = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("deserialize: {e}"),
        };
        assert_eq!(deserialized, original);
        assert!(json.contains("\"rank\":2"));
        assert!(json.contains("\"score\":1"));
        assert!(json.contains("\"rowIds\":[42]"));
    }

    // --- Live hotspot server contract types ---

    fn make_sample_trace_event() -> TraceEvent {
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-24T10:00:00Z".into(),
            session_id: "sess-1".into(),
            event_type: TraceEventType::FileOpened,
            tool_name: Some("read".into()),
            payload: TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/main.rs".into(),
            }),
            outcome: Outcome::Success,
        }
    }

    #[test]
    fn server_ingest_envelope_round_trips() {
        let inner_event = make_sample_trace_event();
        let envelope = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "github.com/scryrs-project/scryrs".into(),
            workspace_id: "ws-abc123".into(),
            agent_id: "pi".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "2026-06-24T10:00:05Z".into(),
                event: inner_event.clone(),
            }],
        };
        let json = serialize_json(&envelope);
        let reconstructed: ServerIngestEnvelope = deserialize_json(&json);
        assert_eq!(reconstructed, envelope);
        assert_eq!(reconstructed.events[0].event, inner_event);
    }

    #[test]
    fn envelope_event_round_trips_independent_of_inner_trace_event() {
        let inner = make_sample_trace_event();
        let env_event = EnvelopeEvent {
            producer_event_id: "evt-002".into(),
            client_timestamp: "2026-06-24T10:01:00Z".into(),
            event: inner.clone(),
        };
        let json = serialize_json(&env_event);
        let reconstructed: EnvelopeEvent = deserialize_json(&json);
        assert_eq!(reconstructed, env_event);
        // Inner TraceEvent round-trips unchanged.
        assert_eq!(reconstructed.event, inner);
    }

    #[test]
    fn batch_ingest_response_round_trips() {
        let response = BatchIngestResponse {
            accepted_count: 2,
            duplicate_count: 1,
            rejected_count: 0,
            received_count: 2,
            events: vec![
                EventAck {
                    index: 0,
                    producer_event_id: Some("evt-001".into()),
                    status: EventAckStatus::Accepted,
                    server_event_id: Some("srv-42".into()),
                    error_reason: None,
                    received_at: "2026-06-24T10:00:07Z".into(),
                },
                EventAck {
                    index: 1,
                    producer_event_id: Some("evt-001".into()),
                    status: EventAckStatus::Idempotent,
                    server_event_id: None,
                    error_reason: None,
                    received_at: "2026-06-24T10:00:07Z".into(),
                },
                EventAck {
                    index: 2,
                    producer_event_id: Some("evt-003".into()),
                    status: EventAckStatus::Accepted,
                    server_event_id: Some("srv-43".into()),
                    error_reason: None,
                    received_at: "2026-06-24T10:00:07Z".into(),
                },
            ],
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&response);
        let reconstructed: BatchIngestResponse = deserialize_json(&json);
        assert_eq!(reconstructed, response);
    }

    #[test]
    fn event_ack_status_serializes_as_snake_case() {
        let ack = EventAck {
            index: 0,
            producer_event_id: Some("evt-001".into()),
            status: EventAckStatus::Accepted,
            error_reason: None,
            server_event_id: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        assert!(json.contains("\"status\":\"accepted\""));

        let ack = EventAck {
            index: 1,
            producer_event_id: Some("evt-001".into()),
            status: EventAckStatus::Idempotent,
            error_reason: None,
            server_event_id: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        assert!(json.contains("\"status\":\"idempotent\""));

        let ack = EventAck {
            index: 2,
            producer_event_id: Some("evt-001".into()),
            status: EventAckStatus::Rejected,
            error_reason: Some("invalid TraceEvent: missing session_id".into()),
            server_event_id: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        assert!(json.contains("\"status\":\"rejected\""));
    }

    #[test]
    fn event_ack_server_event_id_is_optional() {
        let ack_without = EventAck {
            index: 0,
            producer_event_id: Some("evt-001".into()),
            status: EventAckStatus::Accepted,
            error_reason: None,
            server_event_id: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack_without);
        assert!(!json.contains("server_event_id"));

        let ack_with = EventAck {
            index: 0,
            producer_event_id: Some("evt-001".into()),
            status: EventAckStatus::Accepted,
            error_reason: None,
            server_event_id: Some("srv-42".into()),
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack_with);
        assert!(json.contains("server_event_id"));
    }

    #[test]
    fn live_hotspots_response_round_trips() {
        let mut event_type_counts = HashMap::new();
        event_type_counts.insert("FileOpened".to_string(), 3u32);
        let mut outcome_counts = HashMap::new();
        outcome_counts.insert("success".to_string(), 3u32);

        let entry = HotspotEntry {
            rank: 1,
            subjectKind: "file".to_string(),
            subject: "src/main.rs".to_string(),
            score: 15,
            counts: HotspotCounts {
                eventType: event_type_counts,
                outcome: outcome_counts,
            },
            sessionCount: 2,
            firstSeen: "2026-06-24T09:00:00Z".to_string(),
            lastSeen: "2026-06-24T12:00:00Z".to_string(),
            evidence: HotspotEvidence {
                rowIds: vec![10, 20, 30],
            },
        };

        let response = LiveHotspotsResponse {
            schemaVersion: LIVE_HOTSPOT_SCHEMA_VERSION.into(),
            repositoryId: "github.com/scryrs-project/scryrs".into(),
            cursor: "cursor-42".into(),
            generatedAt: "2026-06-24T12:00:00Z".into(),
            entries: vec![entry.clone()],
        };

        let json = serialize_json(&response);
        let reconstructed: LiveHotspotsResponse = deserialize_json(&json);
        assert_eq!(reconstructed, response);
    }

    #[test]
    fn live_hotspots_response_no_filesystem_fields() {
        let response = LiveHotspotsResponse {
            schemaVersion: LIVE_HOTSPOT_SCHEMA_VERSION.into(),
            repositoryId: "github.com/scryrs-project/scryrs".into(),
            cursor: "cursor-1".into(),
            generatedAt: "2026-06-24T12:00:00Z".into(),
            entries: vec![],
        };
        let json = serialize_json(&response);
        // Must not contain local-filesystem fields.
        assert!(!json.contains("repositoryPath"));
        assert!(!json.contains("storePath"));
    }

    #[test]
    fn live_hotspot_schema_version_is_independent() {
        assert_eq!(LIVE_HOTSPOT_SCHEMA_VERSION, "1.0.0");
        assert_ne!(LIVE_HOTSPOT_SCHEMA_VERSION, SCHEMA_VERSION);
        // Live version is the same value as HOTSPOT_SCHEMA_VERSION but semantically independent.
        assert_eq!(LIVE_HOTSPOT_SCHEMA_VERSION, HOTSPOT_SCHEMA_VERSION);
    }

    #[test]
    fn dedup_key_is_4_tuple_of_identity_fields() {
        // Verify the four identity fields that compose the deduplication key exist.
        let inner = make_sample_trace_event();
        let env1 = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-a".into(),
            workspace_id: "ws-1".into(),
            agent_id: "pi".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "2026-06-24T10:00:00Z".into(),
                event: inner.clone(),
            }],
        };

        // Same 4-tuple — should be recognized as same key.
        let env2_same_key = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-a".into(),
            workspace_id: "ws-1".into(),
            agent_id: "pi".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "2026-06-24T10:05:00Z".into(), // different timestamp does not change key
                event: inner.clone(),
            }],
        };

        // Different agent_id — different key.
        let env3_diff_agent = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-a".into(),
            workspace_id: "ws-1".into(),
            agent_id: "claude-code".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "2026-06-24T10:00:00Z".into(),
                event: inner.clone(),
            }],
        };

        // Different repository_id — different key.
        let env4_diff_repo = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-b".into(),
            workspace_id: "ws-1".into(),
            agent_id: "pi".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "2026-06-24T10:00:00Z".into(),
                event: inner.clone(),
            }],
        };

        // Same repository+workspace+agent, different producer_event_id — different key.
        let env5_diff_producer = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-a".into(),
            workspace_id: "ws-1".into(),
            agent_id: "pi".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-002".into(),
                client_timestamp: "2026-06-24T10:00:00Z".into(),
                event: inner.clone(),
            }],
        };

        // Matching 4-tuple should be equal across all identity fields.
        assert_eq!(
            env1.repository_id, env2_same_key.repository_id,
            "same repository_id for same key"
        );
        assert_eq!(
            env1.workspace_id, env2_same_key.workspace_id,
            "same workspace_id for same key"
        );
        assert_eq!(
            env1.agent_id, env2_same_key.agent_id,
            "same agent_id for same key"
        );
        assert_eq!(
            env1.events[0].producer_event_id, env2_same_key.events[0].producer_event_id,
            "same producer_event_id for same key"
        );

        // Different agent_id produces different key.
        assert_ne!(env1.agent_id, env3_diff_agent.agent_id);
        // Different repository_id produces different key.
        assert_ne!(env1.repository_id, env4_diff_repo.repository_id);
        // Different producer_event_id produces different key.
        assert_ne!(
            env1.events[0].producer_event_id,
            env5_diff_producer.events[0].producer_event_id
        );
    }

    #[test]
    fn server_ingest_envelope_with_empty_events_array() {
        let envelope = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-a".into(),
            workspace_id: "ws-1".into(),
            agent_id: "pi".into(),
            events: vec![],
        };
        let json = serialize_json(&envelope);
        let reconstructed: ServerIngestEnvelope = deserialize_json(&json);
        assert_eq!(reconstructed, envelope);
        assert!(reconstructed.events.is_empty());
    }

    #[test]
    fn batch_ingest_response_empty_events() {
        let response = BatchIngestResponse {
            accepted_count: 0,
            duplicate_count: 0,
            rejected_count: 0,
            received_count: 0,
            events: vec![],
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&response);
        let reconstructed: BatchIngestResponse = deserialize_json(&json);
        assert_eq!(reconstructed, response);
        assert!(reconstructed.events.is_empty());
    }

    #[test]
    fn live_hotspots_response_with_empty_entries() {
        let response = LiveHotspotsResponse {
            schemaVersion: LIVE_HOTSPOT_SCHEMA_VERSION.into(),
            repositoryId: "unknown-repo".into(),
            cursor: "cursor-0".into(),
            generatedAt: "2026-06-24T12:00:00Z".into(),
            entries: vec![],
        };
        let json = serialize_json(&response);
        let reconstructed: LiveHotspotsResponse = deserialize_json(&json);
        assert_eq!(reconstructed, response);
        assert!(reconstructed.entries.is_empty());
    }

    #[test]
    fn event_ack_rejected_variant_round_trips() {
        let ack = EventAck {
            index: 0,
            producer_event_id: Some("evt-099".into()),
            status: EventAckStatus::Rejected,
            server_event_id: None,
            error_reason: Some("invalid TraceEvent: missing session_id".into()),
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        let reconstructed: EventAck = deserialize_json(&json);
        assert_eq!(reconstructed, ack);
        assert!(json.contains("\"status\":\"rejected\""));
        assert!(json.contains("error_reason"));
    }

    #[test]
    fn event_ack_rejected_variant_error_reason_is_optional() {
        // error_reason should be omitted when None
        let ack_no_reason = EventAck {
            index: 0,
            producer_event_id: Some("evt-099".into()),
            status: EventAckStatus::Rejected,
            server_event_id: None,
            error_reason: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack_no_reason);
        assert!(!json.contains("error_reason"));

        // error_reason should be present when Some
        let ack_with_reason = EventAck {
            index: 1,
            producer_event_id: Some("evt-099".into()),
            status: EventAckStatus::Rejected,
            server_event_id: None,
            error_reason: Some("validation failed".into()),
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack_with_reason);
        assert!(json.contains("error_reason"));
    }

    // --- Extended EventAck: index field ---

    #[test]
    fn event_ack_includes_request_index_in_serialization() {
        let ack = EventAck {
            index: 42,
            producer_event_id: Some("evt-042".into()),
            status: EventAckStatus::Accepted,
            server_event_id: None,
            error_reason: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        assert!(
            json.contains("\"index\":42"),
            "serialized EventAck must include index field"
        );
    }

    #[test]
    fn event_ack_index_round_trips() {
        let ack = EventAck {
            index: 7,
            producer_event_id: Some("evt-007".into()),
            status: EventAckStatus::Accepted,
            server_event_id: None,
            error_reason: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        let reconstructed: EventAck = deserialize_json(&json);
        assert_eq!(reconstructed.index, 7);
    }

    // --- Extended EventAck: optional producer_event_id ---

    #[test]
    fn event_ack_with_absent_producer_event_id_serializes_without_it() {
        let ack = EventAck {
            index: 0,
            producer_event_id: None,
            status: EventAckStatus::Rejected,
            server_event_id: None,
            error_reason: Some("malformed request item".into()),
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        // The field "producer_event_id" must not appear as a JSON key.
        // Parse to Value and check keys.
        let parsed: serde_json::Value =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        let obj = parsed
            .as_object()
            .unwrap_or_else(|| panic!("expected JSON object"));
        assert!(
            !obj.contains_key("producer_event_id"),
            "serialized JSON object must NOT have producer_event_id key when absent: {json}"
        );
    }

    #[test]
    fn event_ack_with_absent_producer_event_id_round_trips() {
        let ack = EventAck {
            index: 1,
            producer_event_id: None,
            status: EventAckStatus::Rejected,
            server_event_id: None,
            error_reason: Some("missing producer_event_id".into()),
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        let reconstructed: EventAck = deserialize_json(&json);
        assert_eq!(reconstructed, ack);
        assert!(reconstructed.producer_event_id.is_none());
    }

    // --- Extended BatchIngestResponse: accepted_count, rejected_count ---

    #[test]
    fn batch_ingest_response_includes_accepted_and_rejected_counts() {
        let response = BatchIngestResponse {
            accepted_count: 3,
            duplicate_count: 1,
            rejected_count: 2,
            received_count: 3,
            events: vec![
                EventAck {
                    index: 0,
                    producer_event_id: Some("evt-001".into()),
                    status: EventAckStatus::Accepted,
                    server_event_id: Some("srv-1".into()),
                    error_reason: None,
                    received_at: "2026-06-24T10:00:07Z".into(),
                },
                EventAck {
                    index: 1,
                    producer_event_id: None,
                    status: EventAckStatus::Rejected,
                    server_event_id: None,
                    error_reason: Some("invalid TraceEvent".into()),
                    received_at: "2026-06-24T10:00:07Z".into(),
                },
            ],
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&response);
        assert!(
            json.contains("\"accepted_count\":3"),
            "serialized BatchIngestResponse must include accepted_count"
        );
        assert!(
            json.contains("\"duplicate_count\":1"),
            "serialized BatchIngestResponse must include duplicate_count"
        );
        assert!(
            json.contains("\"rejected_count\":2"),
            "serialized BatchIngestResponse must include rejected_count"
        );
        assert!(
            json.contains("\"received_count\":3"),
            "serialized BatchIngestResponse must include received_count"
        );
    }

    #[test]
    fn batch_ingest_response_counts_round_trip() {
        let response = BatchIngestResponse {
            accepted_count: 5,
            duplicate_count: 3,
            rejected_count: 1,
            received_count: 5,
            events: vec![],
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&response);
        let reconstructed: BatchIngestResponse = deserialize_json(&json);
        assert_eq!(reconstructed, response);
        assert_eq!(reconstructed.accepted_count, 5);
        assert_eq!(reconstructed.rejected_count, 1);
    }

    // --- HotspotSignal round-trip tests ---

    #[test]
    fn hotspot_signal_round_trips() {
        let signal = HotspotSignal {
            repositoryId: "repo-a".into(),
            subjectKind: "file".into(),
            subject: "src/main.rs".into(),
            score: 15,
            delta: 3,
            window: "cumulative".into(),
            threshold: 10,
            evidenceRowIds: vec![1, 5, 9],
            createdAt: "2026-06-25T12:00:00Z".into(),
        };
        let json = serialize_json(&signal);
        let reconstructed: HotspotSignal = deserialize_json(&json);
        assert_eq!(reconstructed, signal);
        assert!(json.contains("\"subjectKind\":\"file\""));
        assert!(json.contains("\"window\":\"cumulative\""));
        assert!(json.contains("\"threshold\":10"));
        assert!(json.contains("\"evidenceRowIds\":[1,5,9]"));
    }

    #[test]
    fn hotspot_signal_empty_evidence_row_ids() {
        let signal = HotspotSignal {
            repositoryId: "repo-a".into(),
            subjectKind: "search".into(),
            subject: "routing".into(),
            score: 0,
            delta: 0,
            window: "cumulative".into(),
            threshold: 10,
            evidenceRowIds: vec![],
            createdAt: "2026-06-25T12:00:00Z".into(),
        };
        let json = serialize_json(&signal);
        let reconstructed: HotspotSignal = deserialize_json(&json);
        assert_eq!(reconstructed, signal);
    }

    #[test]
    fn hotspot_signal_delta_independent_of_score() {
        // Signal score and delta are separate fields: score is cumulative, delta is per-event.
        let signal = HotspotSignal {
            repositoryId: "repo-a".into(),
            subjectKind: "command".into(),
            subject: "cargo test".into(),
            score: 12,
            delta: 3,
            window: "cumulative".into(),
            threshold: 10,
            evidenceRowIds: vec![42],
            createdAt: "2026-06-25T12:00:00Z".into(),
        };
        assert_eq!(signal.score, 12);
        assert_eq!(signal.delta, 3);
    }

    // --- Graph contract DTO round-trip tests (Task 3.1) ---

    #[test]
    fn graph_schema_version_is_independent() {
        assert_eq!(GRAPH_SCHEMA_VERSION, "1.0.0");
        assert_ne!(GRAPH_SCHEMA_VERSION, SCHEMA_VERSION);
        // Same string value as hotspot versions but semantically independent.
        assert_eq!(GRAPH_SCHEMA_VERSION, HOTSPOT_SCHEMA_VERSION);
    }

    #[test]
    fn evidence_source_kind_serializes_as_snake_case() {
        let json = serialize_json(&EvidenceSourceKind::HotspotSubject);
        assert!(json.contains("\"hotspot_subject\""), "got: {json}");
        let json = serialize_json(&EvidenceSourceKind::LocalTraceRow);
        assert!(json.contains("\"local_trace_row\""), "got: {json}");
        let json = serialize_json(&EvidenceSourceKind::ServerTraceRow);
        assert!(json.contains("\"server_trace_row\""), "got: {json}");
        let json = serialize_json(&EvidenceSourceKind::DocReference);
        assert!(json.contains("\"doc_reference\""), "got: {json}");
        let json = serialize_json(&EvidenceSourceKind::RecordedEvidence);
        assert!(json.contains("\"recorded_evidence\""), "got: {json}");
    }

    #[test]
    fn evidence_source_kind_round_trips() {
        for kind in &[
            EvidenceSourceKind::HotspotSubject,
            EvidenceSourceKind::LocalTraceRow,
            EvidenceSourceKind::ServerTraceRow,
            EvidenceSourceKind::DocReference,
            EvidenceSourceKind::RecordedEvidence,
        ] {
            let json = serialize_json(kind);
            let reconstructed: EvidenceSourceKind = deserialize_json(&json);
            assert_eq!(reconstructed, *kind);
        }
    }

    #[test]
    fn evidence_link_minimal_round_trips() {
        let link = EvidenceLink {
            source_kind: EvidenceSourceKind::HotspotSubject,
            subject: "src/main.rs".into(),
            row_ids: vec![],
            doc_ref: None,
            description: None,
            score: None,
            metadata: None,
        };
        let json = serialize_json(&link);
        let reconstructed: EvidenceLink = deserialize_json(&json);
        assert_eq!(reconstructed.subject, "src/main.rs");
        assert_eq!(
            reconstructed.source_kind,
            EvidenceSourceKind::HotspotSubject
        );
        assert!(reconstructed.row_ids.is_empty());
        // Optional fields should be absent from JSON when None/empty.
        assert!(!json.contains("docRef"));
        assert!(!json.contains("description"));
        assert!(!json.contains("score"));
        assert!(!json.contains("rowIds"));
    }

    #[test]
    fn evidence_link_with_all_fields_round_trips() {
        let link = EvidenceLink {
            source_kind: EvidenceSourceKind::LocalTraceRow,
            subject: "search:routing".into(),
            row_ids: vec![5, 12, 23],
            doc_ref: Some("docs/routing.md".into()),
            description: Some("traced from 3 event rows".into()),
            score: Some(15),
            metadata: None,
        };
        let json = serialize_json(&link);
        let reconstructed: EvidenceLink = deserialize_json(&json);
        assert_eq!(reconstructed.subject, "search:routing");
        assert_eq!(reconstructed.source_kind, EvidenceSourceKind::LocalTraceRow);
        assert_eq!(reconstructed.row_ids, vec![5, 12, 23]);
        assert_eq!(reconstructed.doc_ref.as_deref(), Some("docs/routing.md"));
        assert_eq!(
            reconstructed.description.as_deref(),
            Some("traced from 3 event rows")
        );
        assert_eq!(reconstructed.score, Some(15));
    }

    #[test]
    fn evidence_link_doc_reference_fields_are_independent() {
        let link = EvidenceLink {
            source_kind: EvidenceSourceKind::DocReference,
            subject: "routing".into(),
            row_ids: vec![],
            doc_ref: Some("docs/routing.md".into()),
            description: None,
            score: None,
            metadata: None,
        };
        let json = serialize_json(&link);
        assert!(json.contains("\"sourceKind\":\"doc_reference\""));
        assert!(json.contains("\"docRef\":\"docs/routing.md\""));
        let reconstructed: EvidenceLink = deserialize_json(&json);
        assert_eq!(reconstructed.source_kind, EvidenceSourceKind::DocReference);
        assert_eq!(reconstructed.doc_ref.as_deref(), Some("docs/routing.md"));
        assert!(reconstructed.row_ids.is_empty());
    }

    #[test]
    fn evidence_link_recorded_evidence_fields_are_independent() {
        let link = EvidenceLink {
            source_kind: EvidenceSourceKind::RecordedEvidence,
            subject: "hotspot:file:src/lib.rs".into(),
            row_ids: vec![],
            doc_ref: None,
            description: Some("manual review: frequent edits".into()),
            score: None,
            metadata: None,
        };
        let json = serialize_json(&link);
        assert!(json.contains("\"sourceKind\":\"recorded_evidence\""));
        assert!(json.contains("\"description\":\"manual review: frequent edits\""));
        let reconstructed: EvidenceLink = deserialize_json(&json);
        assert_eq!(
            reconstructed.source_kind,
            EvidenceSourceKind::RecordedEvidence
        );
        assert_eq!(
            reconstructed.description.as_deref(),
            Some("manual review: frequent edits")
        );
        assert!(reconstructed.row_ids.is_empty());
    }

    #[test]
    fn graph_node_minimal_round_trips() {
        let node = GraphNode {
            id: "n1".into(),
            label: "Node One".into(),
            description: None,
            kind: "concept".into(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![],
            metadata: None,
        };
        let json = serialize_json(&node);
        let reconstructed: GraphNode = deserialize_json(&json);
        assert_eq!(reconstructed, node);
        assert!(json.contains("\"id\":\"n1\""));
        assert!(json.contains("\"kind\":\"concept\""));
        assert!(!json.contains("description"));
        assert!(!json.contains("tags"));
    }

    #[test]
    fn graph_node_full_round_trips() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "example_com_priority".into(),
            serde_json::Value::Number(1.into()),
        );

        let node = GraphNode {
            id: "n1".into(),
            label: "Node One".into(),
            description: Some("A detailed description".into()),
            kind: "file".into(),
            tags: vec!["rust".into(), "core".into()],
            aliases: vec!["n1alias".into()],
            evidence_links: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::HotspotSubject,
                subject: "src/main.rs".into(),
                row_ids: vec![10, 20],
                doc_ref: None,
                description: None,
                score: None,
                metadata: None,
            }],
            metadata: Some(metadata),
        };
        let json = serialize_json(&node);
        let reconstructed: GraphNode = deserialize_json(&json);
        assert_eq!(reconstructed, node);
    }

    #[test]
    fn graph_edge_minimal_round_trips() {
        let edge = GraphEdge {
            id: "e1".into(),
            source_node_id: "n1".into(),
            target_node_id: "n2".into(),
            relationship: "depends_on".into(),
            label: None,
            tags: vec![],
            evidence_links: vec![],
            metadata: None,
        };
        let json = serialize_json(&edge);
        let reconstructed: GraphEdge = deserialize_json(&json);
        assert_eq!(reconstructed, edge);
        assert!(json.contains("\"id\":\"e1\""));
        assert!(json.contains("\"sourceNodeId\":\"n1\""));
        assert!(json.contains("\"targetNodeId\":\"n2\""));
        assert!(json.contains("\"relationship\":\"depends_on\""));
    }

    #[test]
    fn graph_edge_full_round_trips() {
        let edge = GraphEdge {
            id: "e1".into(),
            source_node_id: "n1".into(),
            target_node_id: "n2".into(),
            relationship: "references".into(),
            label: Some("docs reference".into()),
            tags: vec!["cross-crate".into()],
            evidence_links: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::DocReference,
                subject: "routing".into(),
                row_ids: vec![],
                doc_ref: Some("docs/routing.md".into()),
                description: None,
                score: None,
                metadata: None,
            }],
            metadata: None,
        };
        let json = serialize_json(&edge);
        let reconstructed: GraphEdge = deserialize_json(&json);
        assert_eq!(reconstructed, edge);
    }

    #[test]
    fn graph_metadata_round_trips() {
        let meta = GraphMetadata {
            repository_id: Some("github.com/example/repo".into()),
            metadata: None,
        };
        let json = serialize_json(&meta);
        let reconstructed: GraphMetadata = deserialize_json(&json);
        assert_eq!(reconstructed, meta);

        let meta_no_repo = GraphMetadata {
            repository_id: None,
            metadata: None,
        };
        let json = serialize_json(&meta_no_repo);
        assert!(!json.contains("repositoryId"));
        let reconstructed: GraphMetadata = deserialize_json(&json);
        assert!(reconstructed.repository_id.is_none());
    }

    #[test]
    fn knowledge_graph_document_round_trips() {
        let doc = KnowledgeGraphDocument {
            schema_version: GRAPH_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: Some("github.com/example/repo".into()),
                metadata: None,
            },
            nodes: vec![GraphNode {
                id: "n1".into(),
                label: "Node One".into(),
                description: None,
                kind: "file".into(),
                tags: vec![],
                aliases: vec![],
                evidence_links: vec![],
                metadata: None,
            }],
            edges: vec![],
        };
        let json = serialize_json(&doc);
        let reconstructed: KnowledgeGraphDocument = deserialize_json(&json);
        assert_eq!(reconstructed, doc);
        assert!(json.contains("\"schemaVersion\":\"1.0.0\""));
        assert!(json.contains("\"repositoryId\":\"github.com/example/repo\""));
    }

    #[test]
    fn knowledge_graph_document_top_level_fields_are_present() {
        let doc = KnowledgeGraphDocument {
            schema_version: GRAPH_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            nodes: vec![],
            edges: vec![],
        };
        let json = serialize_json(&doc);
        assert!(
            json.contains("\"schemaVersion\""),
            "must contain schemaVersion"
        );
        assert!(json.contains("\"metadata\""), "must contain metadata");
        assert!(json.contains("\"nodes\""), "must contain nodes");
        assert!(json.contains("\"edges\""), "must contain edges");
    }

    // --- Evidence-link compatibility from hotspot contracts (Task 3.4) ---

    #[test]
    fn evidence_link_from_local_hotspot_entry_evidence() {
        // Local HotspotEntry.evidence.rowIds maps to EvidenceLink with sourceKind=local_trace_row.
        let hotspot_evidence = HotspotEvidence {
            rowIds: vec![3, 7, 12, 45, 67],
        };
        let link = EvidenceLink {
            source_kind: EvidenceSourceKind::LocalTraceRow,
            subject: "file:src/main.rs".into(),
            row_ids: hotspot_evidence.rowIds.clone(),
            doc_ref: None,
            description: None,
            score: None,
            metadata: None,
        };
        assert_eq!(link.source_kind, EvidenceSourceKind::LocalTraceRow);
        assert_eq!(link.row_ids, vec![3, 7, 12, 45, 67]);
        // Verify the row IDs were taken from the hotspot evidence unmodified.
        assert_eq!(link.row_ids, hotspot_evidence.rowIds);
    }

    #[test]
    fn evidence_link_from_hotspot_signal_evidence_row_ids() {
        // Live HotspotSignal.evidenceRowIds maps to EvidenceLink with sourceKind=server_trace_row.
        let signal = HotspotSignal {
            repositoryId: "repo-a".into(),
            subjectKind: "file".into(),
            subject: "src/main.rs".into(),
            score: 15,
            delta: 3,
            window: "cumulative".into(),
            threshold: 10,
            evidenceRowIds: vec![1, 5, 9],
            createdAt: "2026-06-25T12:00:00Z".into(),
        };
        let link = EvidenceLink {
            source_kind: EvidenceSourceKind::ServerTraceRow,
            subject: signal.subject.clone(),
            row_ids: signal.evidenceRowIds.clone(),
            doc_ref: None,
            description: None,
            score: Some(signal.score),
            metadata: None,
        };
        assert_eq!(link.source_kind, EvidenceSourceKind::ServerTraceRow);
        assert_eq!(link.subject, "src/main.rs");
        assert_eq!(link.row_ids, vec![1, 5, 9]);
        assert_eq!(link.score, Some(15));
        // Verify the evidence row IDs were taken from the signal unmodified.
        assert_eq!(link.row_ids, signal.evidenceRowIds);
    }

    #[test]
    fn evidence_link_compatibility_does_not_change_hotspot_contracts() {
        // Construct a HotspotEntry and HotspotSignal normally and verify
        // they serialize unchanged — evidence links are additive, not mutating.
        let entry = HotspotEntry {
            rank: 1,
            subjectKind: "file".to_string(),
            subject: "src/lib.rs".to_string(),
            score: 5,
            counts: HotspotCounts {
                eventType: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("FileOpened".to_string(), 1u32);
                    m
                },
                outcome: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("success".to_string(), 1u32);
                    m
                },
            },
            sessionCount: 1,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T09:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: vec![42] },
        };
        let entry_json = serialize_json(&entry);
        assert!(entry_json.contains("\"rank\":1"));
        assert!(entry_json.contains("\"evidence\""));
        assert!(entry_json.contains("\"rowIds\":[42]"));

        let signal = HotspotSignal {
            repositoryId: "repo-a".into(),
            subjectKind: "command".into(),
            subject: "cargo build".into(),
            score: 3,
            delta: 2,
            window: "cumulative".into(),
            threshold: 5,
            evidenceRowIds: vec![77],
            createdAt: "2026-06-25T12:00:00Z".into(),
        };
        let signal_json = serialize_json(&signal);
        assert!(signal_json.contains("\"evidenceRowIds\":[77]"));
        assert!(signal_json.contains("\"subjectKind\":\"command\""));
    }

    // --- Route manifest schema tests ---

    #[test]
    fn route_manifest_document_round_trip() {
        let doc = RouteManifestDocument {
            schema_version: ROUTE_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            routes: vec![RouteEntry {
                id: "file:src/main.rs".into(),
                subject_kind: "file".into(),
                subject: "src/main.rs".into(),
                label: "src/main.rs".into(),
                target: "file:src/main.rs".into(),
                kind: "file".into(),
                evidence_links: vec![EvidenceLink {
                    source_kind: EvidenceSourceKind::LocalTraceRow,
                    subject: "src/main.rs".into(),
                    row_ids: vec![1, 2],
                    doc_ref: None,
                    description: None,
                    score: Some(10),
                    metadata: None,
                }],
                grouping: None,
                metadata: None,
            }],
        };

        let json = serialize_json(&doc);
        assert!(json.contains("\"schemaVersion\":\"1.0.0\""));
        assert!(json.contains("\"id\":\"file:src/main.rs\""));
        assert!(json.contains("\"subjectKind\":\"file\""));
        assert!(json.contains("\"evidenceLinks\""));
        assert!(!json.contains("\"grouping\""));

        let reconstructed: RouteManifestDocument = deserialize_json(&json);
        assert_eq!(reconstructed, doc);
    }

    #[test]
    fn route_entry_with_grouping_round_trip() {
        let entry = RouteEntry {
            id: "doc_page:graph".into(),
            subject_kind: "doc_page".into(),
            subject: "graph".into(),
            label: "graph".into(),
            target: "doc_page:graph".into(),
            kind: "doc_page".into(),
            evidence_links: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::DocReference,
                subject: "graph".into(),
                row_ids: vec![],
                doc_ref: Some("graph".into()),
                description: None,
                score: None,
                metadata: None,
            }],
            grouping: Some(RouteGrouping {
                group_id: "technical".into(),
                group_label: "Technical".into(),
            }),
            metadata: None,
        };

        let json = serialize_json(&entry);
        assert!(json.contains("\"grouping\""));
        assert!(json.contains("\"groupId\":\"technical\""));
        assert!(json.contains("\"groupLabel\":\"Technical\""));
        assert!(json.contains("\"sourceKind\":\"doc_reference\""));

        let reconstructed: RouteEntry = deserialize_json(&json);
        assert_eq!(reconstructed, entry);
    }

    #[test]
    fn route_entry_without_grouping_omits_field() {
        let entry = RouteEntry {
            id: "search:routing".into(),
            subject_kind: "search".into(),
            subject: "routing".into(),
            label: "routing".into(),
            target: "search:routing".into(),
            kind: "search".into(),
            evidence_links: vec![],
            grouping: None,
            metadata: None,
        };

        let json = serialize_json(&entry);
        assert!(!json.contains("grouping"));

        let reconstructed: RouteEntry = deserialize_json(&json);
        assert_eq!(reconstructed, entry);
    }

    #[test]
    fn route_entry_empty_evidence_links_omitted() {
        let entry = RouteEntry {
            id: "symbol:MyStruct".into(),
            subject_kind: "symbol".into(),
            subject: "MyStruct".into(),
            label: "MyStruct".into(),
            target: "symbol:MyStruct".into(),
            kind: "symbol".into(),
            evidence_links: vec![],
            grouping: None,
            metadata: None,
        };

        let json = serialize_json(&entry);
        assert!(!json.contains("evidenceLinks"));
        let reconstructed: RouteEntry = deserialize_json(&json);
        assert_eq!(reconstructed, entry);
    }

    #[test]
    fn route_schema_version_is_independent() {
        assert_eq!(ROUTE_SCHEMA_VERSION, "1.0.0");
        // Must exist independently of all other schema versions.
        assert_ne!(ROUTE_SCHEMA_VERSION, SCHEMA_VERSION);
    }

    #[test]
    fn route_manifest_document_multi_entry_round_trip() {
        let doc = RouteManifestDocument {
            schema_version: ROUTE_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: Some("repo-a".into()),
                metadata: None,
            },
            routes: vec![
                RouteEntry {
                    id: "file:aaa.rs".into(),
                    subject_kind: "file".into(),
                    subject: "aaa.rs".into(),
                    label: "aaa.rs".into(),
                    target: "file:aaa.rs".into(),
                    kind: "file".into(),
                    evidence_links: vec![],
                    grouping: None,
                    metadata: None,
                },
                RouteEntry {
                    id: "search:routing".into(),
                    subject_kind: "search".into(),
                    subject: "routing".into(),
                    label: "routing".into(),
                    target: "search:routing".into(),
                    kind: "search".into(),
                    evidence_links: vec![EvidenceLink {
                        source_kind: EvidenceSourceKind::HotspotSubject,
                        subject: "routing".into(),
                        row_ids: vec![5],
                        doc_ref: None,
                        description: None,
                        score: Some(42),
                        metadata: None,
                    }],
                    grouping: None,
                    metadata: None,
                },
            ],
        };

        let json = serialize_json(&doc);
        let reconstructed: RouteManifestDocument = deserialize_json(&json);
        assert_eq!(reconstructed, doc);
        assert_eq!(reconstructed.routes.len(), 2);
        assert_eq!(
            reconstructed.metadata.repository_id.as_deref(),
            Some("repo-a")
        );
    }

    // --- Proposal document contract tests ---

    fn make_evidence_link(subject: &str) -> EvidenceLink {
        EvidenceLink {
            source_kind: EvidenceSourceKind::HotspotSubject,
            subject: subject.into(),
            row_ids: vec![1],
            doc_ref: None,
            description: None,
            score: Some(5),
            metadata: None,
        }
    }

    fn make_valid_proposal(
        target_type: ProposalTargetType,
        content: ProposedContent,
    ) -> ProposalDocument {
        let id = ProposalDocument::compute_id(&target_type, &content)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));
        ProposalDocument {
            schema_version: PROPOSAL_SCHEMA_VERSION.into(),
            id,
            target_type,
            title: "Test proposal".into(),
            rationale: "This is a test rationale".into(),
            proposed_content: content,
            evidence: vec![make_evidence_link("test-subject")],
            created_at: "2026-06-27T12:00:00Z".into(),
        }
    }

    #[test]
    fn proposal_schema_version_is_independent() {
        assert_eq!(PROPOSAL_SCHEMA_VERSION, "1.0.0");
        assert_ne!(PROPOSAL_SCHEMA_VERSION, SCHEMA_VERSION);
        // Same string value as other contract versions but semantically independent.
        assert_eq!(PROPOSAL_SCHEMA_VERSION, HOTSPOT_SCHEMA_VERSION);
    }

    #[test]
    fn proposal_target_type_serializes_as_snake_case() {
        let json = serialize_json(&ProposalTargetType::DocsNote);
        assert!(json.contains("\"docs_note\""), "got: {json}");
        let json = serialize_json(&ProposalTargetType::Adr);
        assert!(json.contains("\"adr\""), "got: {json}");
        let json = serialize_json(&ProposalTargetType::Skill);
        assert!(json.contains("\"skill\""), "got: {json}");
        let json = serialize_json(&ProposalTargetType::DebuggingPlaybook);
        assert!(json.contains("\"debugging_playbook\""), "got: {json}");
        let json = serialize_json(&ProposalTargetType::MemoryPatch);
        assert!(json.contains("\"memory_patch\""), "got: {json}");
        let json = serialize_json(&ProposalTargetType::SemanticGraphGrouping);
        assert!(json.contains("\"semantic_graph_grouping\""), "got: {json}");
    }

    #[test]
    fn proposal_target_type_round_trips() {
        for target_type in &[
            ProposalTargetType::DocsNote,
            ProposalTargetType::Adr,
            ProposalTargetType::Skill,
            ProposalTargetType::DebuggingPlaybook,
            ProposalTargetType::MemoryPatch,
            ProposalTargetType::SemanticGraphGrouping,
        ] {
            let json = serialize_json(target_type);
            let reconstructed: ProposalTargetType = deserialize_json(&json);
            assert_eq!(reconstructed, *target_type);
        }
    }

    #[test]
    fn proposal_document_round_trip_markdown_target() {
        let content = ProposedContent::Markdown("# Hello\n\nThis is a test.".into());
        let doc = make_valid_proposal(ProposalTargetType::DocsNote, content);

        let json = serialize_json(&doc);
        assert!(json.contains("\"schemaVersion\":\"1.0.0\""));
        assert!(json.contains("\"targetType\":\"docs_note\""));
        assert!(json.contains("\"rationale\":\"This is a test rationale\""));
        assert!(json.contains("\"proposedContent\":\"# Hello"));

        let reconstructed: ProposalDocument = deserialize_json(&json);
        assert_eq!(reconstructed, doc);
    }

    #[test]
    fn proposal_document_round_trip_memory_patch_target() {
        let patch: serde_json::Value = serde_json::json!({"key": "value", "op": "upsert"});
        let content = ProposedContent::MemoryPatch(patch);
        let doc = make_valid_proposal(ProposalTargetType::MemoryPatch, content);

        let json = serialize_json(&doc);
        assert!(json.contains("\"targetType\":\"memory_patch\""));

        let reconstructed: ProposalDocument = deserialize_json(&json);
        assert_eq!(reconstructed, doc);
    }

    #[test]
    fn proposal_document_round_trip_semantic_graph_grouping_target() {
        let grouping = SemanticGraphGrouping {
            source_node_ids: vec![
                "file:auth".into(),
                "search:auth".into(),
                "symbol:auth".into(),
            ],
            target_group_node_id: "domain_term:auth".into(),
            target_group_label: "auth".into(),
        };
        let content = ProposedContent::SemanticGraphGrouping(grouping);
        let doc = make_valid_proposal(ProposalTargetType::SemanticGraphGrouping, content);

        let json = serialize_json(&doc);
        assert!(json.contains("\"targetType\":\"semantic_graph_grouping\""));
        assert!(json.contains("\"sourceNodeIds\":[\"file:auth\",\"search:auth\",\"symbol:auth\"]"));
        assert!(json.contains("\"targetGroupNodeId\":\"domain_term:auth\""));
        assert!(json.contains("\"targetGroupLabel\":\"auth\""));

        let reconstructed: ProposalDocument = deserialize_json(&json);
        assert_eq!(reconstructed, doc);
    }

    #[test]
    fn validate_accepts_valid_markdown_proposal() {
        let content = ProposedContent::Markdown("# Test".into());
        let doc = make_valid_proposal(ProposalTargetType::DocsNote, content);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn validate_accepts_valid_memory_patch_proposal() {
        let patch: serde_json::Value = serde_json::json!({"op": "upsert"});
        let content = ProposedContent::MemoryPatch(patch);
        let doc = make_valid_proposal(ProposalTargetType::MemoryPatch, content);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn validate_accepts_valid_semantic_graph_grouping_proposal() {
        let grouping = SemanticGraphGrouping {
            source_node_ids: vec!["file:auth".into()],
            target_group_node_id: "domain_term:auth".into(),
            target_group_label: "auth".into(),
        };
        let content = ProposedContent::SemanticGraphGrouping(grouping);
        let doc = make_valid_proposal(ProposalTargetType::SemanticGraphGrouping, content);
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn proposal_validate_rejects_wrong_schema_version() {
        let content = ProposedContent::Markdown("# Test".into());
        let mut doc = make_valid_proposal(ProposalTargetType::DocsNote, content);
        doc.schema_version = "0.9.9".into();
        let result = doc.validate();
        assert!(
            result.is_err(),
            "schema version mismatch should be rejected"
        );
        let err = match result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("schema_version mismatch"));
        assert!(err.contains("0.9.9"));
        assert!(err.contains(PROPOSAL_SCHEMA_VERSION));
    }

    #[test]
    fn validate_rejects_empty_rationale() {
        let content = ProposedContent::Markdown("# Test".into());
        let mut doc = make_valid_proposal(ProposalTargetType::Adr, content);
        doc.rationale = String::new();
        let result = doc.validate();
        assert!(result.is_err(), "empty rationale should be rejected");
        let err = match result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("rationale"));
    }

    #[test]
    fn validate_rejects_whitespace_only_rationale() {
        let content = ProposedContent::Markdown("# Test".into());
        let mut doc = make_valid_proposal(ProposalTargetType::Adr, content);
        doc.rationale = "   ".into();
        let result = doc.validate();
        assert!(
            result.is_err(),
            "whitespace-only rationale should be rejected"
        );
    }

    #[test]
    fn validate_rejects_empty_evidence() {
        let content = ProposedContent::Markdown("# Test".into());
        let mut doc = make_valid_proposal(ProposalTargetType::Skill, content);
        doc.evidence = vec![];
        let result = doc.validate();
        assert!(result.is_err(), "empty evidence should be rejected");
        let err = match result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("evidence"));
    }

    #[test]
    fn validate_rejects_empty_markdown_proposed_content() {
        let content = ProposedContent::Markdown(String::new());
        let doc = make_valid_proposal(ProposalTargetType::DebuggingPlaybook, content);
        let result = doc.validate();
        assert!(result.is_err(), "empty markdown content should be rejected");
        let err = match result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("markdown"));
    }

    #[test]
    fn validate_rejects_whitespace_only_markdown_proposed_content() {
        let content = ProposedContent::Markdown("  \n ".into());
        let doc = make_valid_proposal(ProposalTargetType::DocsNote, content);
        let result = doc.validate();
        assert!(
            result.is_err(),
            "whitespace-only markdown should be rejected"
        );
    }

    #[test]
    fn validate_rejects_null_memory_patch_proposed_content() {
        let content = ProposedContent::MemoryPatch(serde_json::Value::Null);
        let doc = make_valid_proposal(ProposalTargetType::MemoryPatch, content);
        let result = doc.validate();
        assert!(result.is_err(), "null memory patch should be rejected");
        let err = match result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("memory_patch"));
    }

    #[test]
    fn validate_rejects_empty_title() {
        let content = ProposedContent::Markdown("# Test".into());
        let mut doc = make_valid_proposal(ProposalTargetType::DocsNote, content);
        doc.title = String::new();
        let result = doc.validate();
        assert!(result.is_err(), "empty title should be rejected");
        let err = match result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("title"));
    }

    #[test]
    fn validate_rejects_whitespace_only_title() {
        let content = ProposedContent::Markdown("# Test".into());
        let mut doc = make_valid_proposal(ProposalTargetType::DocsNote, content);
        doc.title = "  \n ".into();
        let result = doc.validate();
        assert!(result.is_err(), "whitespace-only title should be rejected");
    }

    #[test]
    fn validate_rejects_empty_source_node_ids_for_graph_grouping() {
        let grouping = SemanticGraphGrouping {
            source_node_ids: vec![],
            target_group_node_id: "domain_term:auth".into(),
            target_group_label: "auth".into(),
        };
        let content = ProposedContent::SemanticGraphGrouping(grouping);
        let doc = make_valid_proposal(ProposalTargetType::SemanticGraphGrouping, content);
        let result = doc.validate();
        assert!(result.is_err(), "empty sourceNodeIds should be rejected");
        let err = match result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("sourceNodeIds"));
    }

    #[test]
    fn deterministic_id_produces_stable_fingerprint() {
        let content = ProposedContent::Markdown("# Same Content".into());
        let id1 = ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));
        let id2 = ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));
        assert_eq!(id1, id2, "same target type and content must yield same id");
    }

    #[test]
    fn deterministic_id_differs_by_target_type() {
        let content = ProposedContent::Markdown("# Content".into());
        let id_docs = ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));
        let id_adr = ProposalDocument::compute_id(&ProposalTargetType::Adr, &content)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));
        assert_ne!(
            id_docs, id_adr,
            "different target types must yield different ids"
        );
    }

    #[test]
    fn deterministic_id_differs_by_content() {
        let content1 = ProposedContent::Markdown("# Content A".into());
        let content2 = ProposedContent::Markdown("# Content B".into());
        let id1 = ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content1)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));
        let id2 = ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content2)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));
        assert_ne!(id1, id2, "different content must yield different ids");
    }

    #[test]
    fn deterministic_id_is_hex_string() {
        let content = ProposedContent::Markdown("# Test".into());
        let id = ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));
        // SHA-256 produces 64 hex characters.
        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn inbox_filename_uses_id_with_json_extension() {
        let content = ProposedContent::Markdown("# Test".into());
        let doc = make_valid_proposal(ProposalTargetType::DocsNote, content);
        let filename = doc.inbox_filename();
        assert!(filename.ends_with(".json"));
        assert!(filename.starts_with(&doc.id));
        assert_eq!(filename.len(), doc.id.len() + 5); // id + ".json"
    }

    #[test]
    fn proposal_evidence_uses_evidence_link_vocabulary() {
        let content = ProposedContent::Markdown("# Test".into());
        let doc = make_valid_proposal(ProposalTargetType::DocsNote, content);
        let json = serialize_json(&doc);

        // Evidence fields should use EvidenceLink camelCase serialization.
        assert!(json.contains("\"sourceKind\":\"hotspot_subject\""));
        assert!(json.contains("\"subject\":\"test-subject\""));
        assert!(json.contains("\"rowIds\":[1]"));
        assert!(json.contains("\"score\":5"));
    }

    #[test]
    fn proposal_document_has_no_acceptance_lifecycle_fields() {
        let content = ProposedContent::Markdown("# Test".into());
        let doc = make_valid_proposal(ProposalTargetType::Adr, content);
        let json = serialize_json(&doc);

        // Must NOT contain review lifecycle fields.
        assert!(!json.contains("\"status\""));
        assert!(!json.contains("\"reviewer\""));
        assert!(!json.contains("\"accepted"));
        assert!(!json.contains("\"rejected"));
    }
}
