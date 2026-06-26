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
  scryrs init --agent <NAME>\n\
      Install the scryrs trace hook for a supported agent harness.\n\
      Supported harnesses: claude-code, pi\n\
  scryrs dashboard [--port <PORT>] [--bind <ADDR>] [--no-open] [--dev]\n\
      Start local dashboard server and open the browser dashboard.\n\
  scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]\n\
      Start the central trace ingest server for POST /v1/trace-events/batch.\n\n\
RECORD MODES\n\
  Local mode (default): persisted to .scryrs/scryrs.db, no network calls.\n\
  Remote mode: activated when a non-empty ingest URL is configured.\n\
      Configure via scryrs.json `remote` section, overridden by:\n\
        SCRYRS_REMOTE_INGEST_URL, SCRYRS_REPOSITORY_ID,\n\
        SCRYRS_WORKSPACE_ID, SCRYRS_AGENT_ID, SCRYRS_REMOTE_TIMEOUT_MS.\n\
      Remote mode skips SQLite entirely (no dual-write, no local fallback).\n\
      Default timeout: 3000 ms.\n\n\
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
  scryrs dashboard\n\
  scryrs dashboard --port 9090 --no-open\n\
  scryrs server\n\
  scryrs server --port 9091\n\n\
OPTIONS\n\
  -h, --help       Print this help message and exit\n\
  -V, --version    Print version and exit\n\
  -hj, --help-json Print machine-readable CLI surface description and exit\n\n\
EXIT CODES\n\
  0    Success (hotspots: JSON written; record local: all events accepted; record remote: no rejections or failures; init: hook installed; dashboard: server shut down cleanly; server: server shut down cleanly; hook: always — fail-open, never blocks the harness)\n\
  1    Hotspots: storage error. Record: rejected events or I/O error (local or server rejections). Init: I/O error. Dashboard: port in use or artifact read error. Server: port in use or store error.\n\
  2    Usage error; hotspots: missing/unsupported store; record: also fatal I/O error (unreadable file, store failure, missing remote identity, transport timeout, connection failure, non-2xx response, malformed response); init: unsupported harness, collision, or self-install refusal; dashboard: invalid flags or bind failure; server: invalid flags or bind failure",
        SCHEMA_VERSION, SCHEMA_VERSION, HOTSPOT_SCHEMA_VERSION
    )
}
