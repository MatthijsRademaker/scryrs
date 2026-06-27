use std::io::{self, Write};

use serde_json::json;

/// Version of the `--help-json` surface document format, independent of
/// `SCHEMA_VERSION` which governs command output envelopes.
const SURFACE_VERSION: &str = "0.7.0";

pub(crate) fn cli_surface_doc() -> String {
    let doc = json!({
        "surfaceVersion": SURFACE_VERSION,
        "binary": "scryrs",
        "commands": [
            {
                "name": "hotspots",
                "description": "Discover and analyze knowledge hotspots in a repository",
                "arguments": [
                    {
                        "name": "PATH",
                        "type": "string",
                        "required": true,
                        "description": "Path to the repository root directory"
                    }
                ],
                "output": {
                    "mimeType": "application/json",
                    "fields": [
                        {"name": "schemaVersion", "type": "string", "description": "Version of the hotspot report output format (independent of trace event version)", "optional": false},
                        {"name": "command", "type": "string", "description": "Name of the executed command", "optional": false},
                        {"name": "repositoryPath", "type": "string", "description": "Resolved absolute path to the repository root", "optional": false},
                        {"name": "storePath", "type": "string", "description": "Resolved absolute path to .scryrs/scryrs.db", "optional": false},
                        {"name": "runMetadata", "type": "object", "description": "Deterministic metadata from store state (storeSchemaVersion, analyzedEventCount, analyzedSubjectCount, firstEventId, lastEventId)", "optional": false},
                        {"name": "generatedAt", "type": "string", "description": "ISO 8601 wall-clock timestamp", "optional": false},
                        {"name": "entries", "type": "array", "description": "Array of ranked HotspotEntry objects (empty for stores with no subject-bearing events)", "optional": false}
                    ]
                }
            },
            {
                "name": "record",
                "description": "Ingest JSONL trace events from stdin or file. Defaults to local mode (writes to .scryrs/scryrs.db). Activates remote mode when a non-empty ingest URL is configured via scryrs.json `remote` section (overridden by SCRYRS_REMOTE_INGEST_URL, SCRYRS_REPOSITORY_ID, SCRYRS_WORKSPACE_ID, SCRYRS_AGENT_ID, SCRYRS_REMOTE_TIMEOUT_MS). Remote mode skips local SQLite and submits a single batch to POST /v1/trace-events/batch.",
                "modes": [
                    {"name": "stdin", "flag": "--stdin", "description": "Read JSONL events from stdin"},
                    {"name": "file", "flag": "--file", "value": "PATH", "description": "Read JSONL events from PATH"}
                ],
                "transport": {
                    "local": {
                        "description": "Default transport — persists accepted events to .scryrs/scryrs.db via the canonical EventStore.",
                        "output": {
                            "mimeType": "application/json",
                            "fields": [
                                {"name": "command", "type": "string", "description": "Name of the executed command (always \"record\")", "optional": false},
                                {"name": "schemaVersion", "type": "string", "description": "Version of the output envelope format", "optional": false},
                                {"name": "accepted", "type": "number", "description": "Count of accepted events", "optional": false},
                                {"name": "rejected", "type": "number", "description": "Count of rejected non-empty lines", "optional": false}
                            ]
                        }
                    },
                    "remote": {
                        "description": "Explicit remote mode — activated by scryrs.json `remote.ingest_url` or SCRYRS_REMOTE_INGEST_URL. Skips .scryrs/scryrs.db entirely. Default timeout 3000 ms.",
                        "configPrecedence": ["1. Environment variables (SCRYRS_REMOTE_*)", "2. scryrs.json `remote` section", "3. Git remote origin URL (repository_id fallback only)"],
                        "requiredIdentity": ["repository_id", "workspace_id", "agent_id"],
                        "output": {
                            "mimeType": "application/json",
                            "fields": [
                                {"name": "command", "type": "string", "description": "Name of the executed command (always \"record\")", "optional": false},
                                {"name": "schemaVersion", "type": "string", "description": "Version of the output envelope format", "optional": false},
                                {"name": "transport", "type": "string", "description": "Always \"remote\" when remote mode is active", "optional": false},
                                {"name": "accepted", "type": "number", "description": "Count of events the server accepted", "optional": false},
                                {"name": "duplicate", "type": "number", "description": "Count of idempotent (previously seen) events — non-fatal", "optional": false},
                                {"name": "rejected", "type": "number", "description": "Count of rejected events (local validation + server rejections)", "optional": false},
                                {"name": "failed", "type": "number", "description": "Count of server-rejected items", "optional": false}
                            ]
                        }
                    }
                },
                "stderr": {
                    "mimeType": "application/jsonl",
                    "description": "One JSON object per rejected non-empty line (local) or per server-rejected item (remote, with line -1 and producer_event_id as the field)",
                    "fields": [
                        {"name": "line", "type": "number", "description": "1‑based physical line number (-1 for server-rejected items)", "optional": false},
                        {"name": "field", "type": "string|null", "description": "Failing field/path when available, or producer_event_id for server rejects", "optional": true},
                        {"name": "reason", "type": "string", "description": "Human-readable rejection reason", "optional": false}
                    ]
                }
            },
            {
                "name": "hook",
                "description": "Translate a harness's native tool event and record it (harness integration entry point; fail-open)",
                "arguments": [
                    {
                        "name": "harness",
                        "type": "string",
                        "required": true,
                        "values": ["claude-code", "pi"],
                        "description": "Harness whose native event is being translated"
                    }
                ],
                "modes": [
                    {"name": "stdin", "flag": "--stdin", "description": "Read the harness event from stdin (default; used by Claude Code)"},
                    {"name": "file", "flag": "--file", "value": "PATH", "description": "Read the harness event from PATH (used by the Pi shim)"}
                ],
                "failOpen": true,
                "output": {
                    "mimeType": "none",
                    "description": "Writes nothing to stdout and always exits 0 (fail-open). Errors are appended to .scryrs/hooks/<harness>-warnings.log; the harness is never blocked."
                }
            },
            {
                "name": "init",
                "description": "Install scryrs trace hook for a supported agent harness",
                "arguments": [
                    {
                        "name": "agent",
                        "flag": "--agent",
                        "type": "string",
                        "required": true,
                        "description": "Agent harness name (claude-code or pi)"
                    },
                    {
                        "name": "mode",
                        "flag": "--mode",
                        "type": "string",
                        "values": ["local", "live"],
                        "default": "local",
                        "description": "Install mode: local for SQLite trace store, live for remote ingest via scryrs server"
                    },
                    {
                        "name": "ingest-url",
                        "flag": "--ingest-url",
                        "type": "string",
                        "description": "Live-mode remote ingest URL (required with --mode live)"
                    },
                    {
                        "name": "workspace-id",
                        "flag": "--workspace-id",
                        "type": "string",
                        "description": "Live-mode workspace identity (required with --mode live)"
                    },
                    {
                        "name": "agent-id",
                        "flag": "--agent-id",
                        "type": "string",
                        "description": "Live-mode agent identity (required with --mode live)"
                    },
                    {
                        "name": "repository-id",
                        "flag": "--repository-id",
                        "type": "string",
                        "description": "Live-mode repository identity (derived from Git remote origin if omitted)"
                    }
                ],
                "output": {
                    "mimeType": "text/plain",
                    "description": "Post-install next-step instructions on stdout. Errors on stderr."
                }
            },
            {
                "name": "dashboard",
                "description": "Start local dashboard server and open the browser dashboard",
                "flags": [
                    {"name": "port", "short": "-p", "long": "--port", "type": "number", "default": 8080, "description": "TCP port to bind"},
                    {"name": "bind", "short": "-b", "long": "--bind", "type": "string", "default": "127.0.0.1", "description": "Bind address"},
                    {"name": "no-open", "long": "--no-open", "type": "boolean", "default": false, "description": "Do not open browser automatically"},
                    {"name": "dev", "long": "--dev", "type": "boolean", "default": false, "description": "Serve from filesystem instead of embedded assets"}
                ],
                "output": {
                    "mimeType": "text/html",
                    "description": "Vue.js SPA served over HTTP. REST API at GET /api/hotspots, GET /api/sessions, GET /api/sessions/:sessionId, GET /api/events."
                }
            },
            {
                "name": "server",
                "description": "Start the central trace ingest server with live hotspot query and signal streaming endpoints",
                "flags": [
                    {"name": "port", "short": "-p", "long": "--port", "type": "number", "default": 8081, "description": "TCP port to bind"},
                    {"name": "bind", "short": "-b", "long": "--bind", "type": "string", "default": "127.0.0.1", "description": "Bind address"},
                    {"name": "store", "long": "--store", "type": "string", "default": ".scryrs/server.db", "description": "Server-owned SQLite store path"}
                ],
                "endpoints": [
                    {"method": "POST", "path": "/v1/trace-events/batch", "description": "Ingest trace event batches with idempotent first-writer-wins semantics"},
                    {"method": "GET", "path": "/v1/repositories/{repository_id}/hotspots", "description": "Query live hotspot rankings from server-owned state; supports ?window=cumulative and optional ?session_id"},
                    {"method": "GET", "path": "/v1/repositories/{repository_id}/signals", "description": "Server-Sent Events stream of HotspotSignal records; supports ?after=<signal_id> cursor replay/resume"}
                ],
                "output": {
                    "mimeType": "application/json",
                    "description": "BatchIngestResponse returned by POST /v1/trace-events/batch. LiveHotspotsResponse returned by GET .../hotspots. text/event-stream returned by GET .../signals."
                }
            },
            {
                "name": "graph",
                "description": "Build a repository knowledge graph from hotspot evidence and docs structure",
                "arguments": [
                    {
                        "name": "PATH",
                        "type": "string",
                        "required": true,
                        "description": "Path to the repository root directory"
                    }
                ],
                "output": {
                    "mimeType": "application/json",
                    "description": "Single-line KnowledgeGraphDocument JSON written to stdout. Also persisted to .scryrs/graph.json."
                }
            }
        ],
        "globalFlags": [
            {"name": "help", "short": "-h", "long": "--help", "description": "Print help message and exit", "action": "help"},
            {"name": "version", "short": "-V", "long": "--version", "description": "Print version and exit", "action": "version"},
            {"name": "help-json", "short": "-hj", "long": "--help-json", "description": "Print machine-readable CLI surface description and exit", "action": "helpJson"}
        ],
        "rootBehavior": {"action": "help", "exitCode": 0},
        "exitCodes": {
            "0": "Success (hotspots: JSON written, including empty entries; record local: all events accepted; record remote: no rejections or failures; init: hook installed; dashboard: server shut down cleanly; server: server shut down cleanly; hook: always — fail-open, never blocks the harness)",
            "1": "Hotspots: storage error. Record: one or more events rejected (local or server), or I/O error writing output. Init: I/O error. Dashboard: port in use or artifact read error. Server: port in use or store error.",
            "2": "Usage error; hotspots: missing/unsupported store; record: also fatal I/O error (unreadable file, store failure, missing remote identity, transport timeout, connection failure, non-2xx response, malformed response); init: unsupported harness, collision, self-install refusal, invalid mode, or missing/invalid live-mode configuration; dashboard: invalid flags; server: invalid flags or bind failure."
        }
    });
    serde_json::to_string(&doc).unwrap_or_else(|_| "{}".into())
}

pub(crate) fn write_cli_surface(out: &mut impl Write) -> io::Result<()> {
    write!(out, "{}", cli_surface_doc())
}
