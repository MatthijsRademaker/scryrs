use std::io::{self, Write};

use scryrs_types::{HOTSPOT_SCHEMA_VERSION, SCHEMA_VERSION};

pub(crate) fn write_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "scryrs — context intelligence for AI-assisted codebases\n\n\
Discover, analyze, and navigate hotspots in your codebase.\n\n\
COMMANDS\n\
  scryrs hotspots <PATH>\n\
      Emit a versioned JSON hotspot report from recorded trace events.\n\
  scryrs record --stdin\n\
      Ingest JSONL trace events from stdin.\n\
  scryrs record --file <PATH>\n\
      Ingest JSONL trace events from a file.\n\
  scryrs hook <HARNESS> [--stdin | --file <PATH>]\n\
      Translate a harness's native tool event and record it (fail-open).\n\
      Supported harnesses: claude-code (stdin), pi (--file).\n\
  scryrs init --agent <NAME> [--mode live|local] [--ingest-url <URL>]\n\
        [--workspace-id <ID>] [--agent-id <ID>] [--repository-id <ID>]\n\
      Install the scryrs trace hook for a supported agent harness.\n\
      Supported harnesses: claude-code, pi\n\
      --mode live (default): remote ingest via scryrs server (writes project scryrs.json\n\
        and a gitignored .scryrs/.env). Identity resolves from flags, then env,\n\
        then .scryrs/.env, then scryrs.json remote; unresolved config fails fast.\n\
        --repository-id is derived from Git remote origin when omitted.\n\
      --mode local: local SQLite trace store (.scryrs/scryrs.db), no network.\n\
  scryrs doctor [--json]\n\
      Run the installation and readiness diagnostic command.\n\
      Reports binary version, command surface, resolved mode, local store,\n\
      hook status, live server reachability when configured, and docs links.\n\
  scryrs graph <PATH>\n\
      Build a repository knowledge graph from hotspot evidence and docs structure.\n\
      Emits a single-line KnowledgeGraphDocument JSON to stdout and .scryrs/graph.json.\n\
  scryrs route <PATH>\n\
      Generate the route manifest from a knowledge graph artifact.\n\
      Emits a single-line RouteManifestDocument JSON to stdout and .scryrs/routes.json.\n\
  scryrs route explain <PATH> --query <TEXT>\n\
      Query the route manifest for matching entries.\n\
      Case-insensitive substring match against label, subject, id, target, kind,\n\
      and evidence_links[].subject. Match tier (exact > prefix > substring) orders\n\
      results. Returns single-line RouteHintDocument JSON. Zero matches produces\n\
      valid document with empty hints array.\n\
      Example: scryrs route explain . --query \"authentication\"\n\
\n\
      Route hint contract: Each route entry projects to a RouteHintItem\n\
      (HINT_SCHEMA_VERSION 1.0.0) with routeId, target, label, 1-based\n\
      ordinal rank, evidence citations, and a template-derived reason.\n\
      The reason field appends \"; query match on <fields>\" for explain results.\n\
      Rank is a deterministic ordinal derived from manifest entry order;\n\
      relevance is deferred (None).\n\
  scryrs propose <PATH>\n\
      Generate reviewable knowledge proposals from hotspot and graph evidence.\n\
      Writes validated ProposalDocument files under .scryrs/proposals/.\n\
      Timestamps derive from the hotspot artifact's generatedAt for determinism.\n\
      Note: singular `propose` generates proposals; plural `proposals` reviews them.\n\
  scryrs proposals list <PATH> [--state pending|accepted|rejected|all]\n\
      List pending and reviewed proposal states from .scryrs/proposals/,\n\
      .scryrs/accepted/, and .scryrs/rejected/ as deterministic JSON.\n\
  scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>\n\
      Accept a validated proposal without mutating the proposal inbox artifact.\n\
  scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>\n\
      Reject a validated proposal without mutating the proposal inbox artifact.\n\
  scryrs dashboard [--mode live|local] [--port <PORT>] [--bind <ADDR>] [--server-url <URL>] [--repository-id <ID>] [--no-open] [--dev]\n\
      Start dashboard server and open the browser dashboard (live by default).\n\
  scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]\n\
      Start the central trace ingest server with live hotspot query
\
      and signal streaming endpoints.\n\n\
RECORD MODES\n\
  Remote mode (default): submits to the configured ingest server.\n\
      Identity resolves by precedence — flags, then environment, then\n\
        .scryrs/.env, then scryrs.json `remote` — using the variables:\n\
        SCRYRS_REMOTE_INGEST_URL, SCRYRS_REPOSITORY_ID,\n\
        SCRYRS_WORKSPACE_ID, SCRYRS_AGENT_ID, SCRYRS_REMOTE_TIMEOUT_MS.\n\
      Remote mode skips SQLite entirely (no dual-write, no local fallback).\n\
      Unresolved required config fails fast (exit 2) with guidance.\n\
      Default timeout: 3000 ms.\n\
  Local mode (--mode local): persisted to .scryrs/scryrs.db, no network calls.\n\n\
RECORD OUTPUT\n\
  Local mode — single-line JSON summary on stdout:\n\
    {{\n\
      \"command\": \"record\",\n\
      \"schemaVersion\": \"{}\",\n\
      \"accepted\": <count>,\n\
      \"rejected\": <count>\n\
    }}\n\
  Remote mode — single-line JSON summary on stdout:\n\
    {{\n\
      \"command\": \"record\",\n\
      \"schemaVersion\": \"{}\",\n\
      \"transport\": \"remote\",\n\
      \"accepted\": <count>,\n\
      \"duplicate\": <count>,\n\
      \"rejected\": <count>,\n\
      \"failed\": <count>\n\
    }}\n\
  Rejection diagnostics are written as JSON objects to stderr,\n\
  one per rejected non-empty line (local) or per server-rejected item (remote).\n\n\
HOTSPOTS OUTPUT\n\
  A single-line JSON envelope on stdout:\n\
    {{\n\
      \"schemaVersion\": \"{}\",\n\
      \"command\": \"hotspots\",\n\
      \"repositoryPath\": \"<absolute path>\",\n\
      \"storePath\": \"<absolute path to .scryrs/scryrs.db>\",\n\
      \"runMetadata\": {{\n\
        \"storeSchemaVersion\": <integer>,\n\
        \"analyzedEventCount\": <count>,\n\
        \"analyzedSubjectCount\": <count>,\n\
        \"firstEventId\": <id>,\n\
        \"lastEventId\": <id>\n\
      }},\n\
      \"generatedAt\": \"<ISO 8601 timestamp>\",\n\
      \"entries\": [...]\n\
    }}\n\
  Each entry carries rank, subjectKind, subject, score,\n\
  per-event-type counts, per-outcome counts, sessionCount,\n\
  firstSeen/lastSeen timestamps, and evidence rowIds.\n\
  Empty stores produce entries: [].\n\
  On success, the report is also written to .scryrs/hotspots.json.\n\
EXAMPLES\n\
  scryrs hotspots /path/to/repo\n\
  scryrs hotspots .\n\
  scryrs record --stdin < events.jsonl\n\
  scryrs record --file session.jsonl\n\
  scryrs hook claude-code < pre-tool-use.json\n\
  scryrs hook pi --file event.json\n\
  scryrs init --agent claude-code\n\
  scryrs init --agent pi\n\
  scryrs init --agent claude-code --mode live --ingest-url http://scryrs-server:8081 --workspace-id my-workspace --agent-id agent-1\n\
  scryrs dashboard\n\
  scryrs dashboard --port 9090 --no-open\n\
  scryrs server\n\
  scryrs server --port 9091\n\
  scryrs graph .
  scryrs route .
  scryrs route explain . --query \"authentication\"\n\n\
OPTIONS\n\
  -h, --help       Print this help message and exit\n\
  -V, --version    Print version and exit\n\
  -hj, --help-json Print machine-readable CLI surface description and exit\n\n\
EXIT CODES\n\
  0    Success (hotspots: JSON written; record local: all events accepted; record remote: no rejections or failures; init: hook installed; doctor: only ok/warn findings; propose/proposals: artifacts written or listed successfully; dashboard: server shut down cleanly; server: server shut down cleanly; hook: always — fail-open, never blocks the harness)\n\
  1    Hotspots: storage error. Record: rejected events or I/O error (local or server rejections). Init: I/O error. Doctor: output write failure. Proposals: serialization or filesystem write failure. Dashboard: port in use or artifact read error. Server: port in use or store error.\n\
  2    Usage error; hotspots: missing/unsupported store; record: also fatal I/O error (unreadable file, store failure, missing remote identity, transport timeout, connection failure, non-2xx response, malformed response); init: unsupported harness, collision, self-install refusal, invalid mode, or missing/invalid live-mode configuration; doctor: one or more structural error findings; proposals: invalid filter, invalid proposal/review document, unknown proposal ID, or conflicting terminal review state; route explain: missing PATH, missing --query, missing/malformed/schema-mismatched routes.json; dashboard: invalid flags, bind failure, or partial live-mode configuration; server: invalid flags or bind failure",
        SCHEMA_VERSION, SCHEMA_VERSION, HOTSPOT_SCHEMA_VERSION
    )
}
