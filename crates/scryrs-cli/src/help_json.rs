use std::io::{self, Write};

use serde_json::json;

/// Version of the `--help-json` surface document format, independent of
/// `SCHEMA_VERSION` which governs command output envelopes.
const SURFACE_VERSION: &str = "0.6.0";

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
                "description": "Ingest JSONL trace events from stdin or file",
                "modes": [
                    {"name": "stdin", "flag": "--stdin", "description": "Read JSONL events from stdin"},
                    {"name": "file", "flag": "--file", "value": "PATH", "description": "Read JSONL events from PATH"}
                ],
                "output": {
                    "mimeType": "application/json",
                    "fields": [
                        {"name": "command", "type": "string", "description": "Name of the executed command (always \"record\")", "optional": false},
                        {"name": "schemaVersion", "type": "string", "description": "Version of the output envelope format", "optional": false},
                        {"name": "accepted", "type": "number", "description": "Count of accepted events", "optional": false},
                        {"name": "rejected", "type": "number", "description": "Count of rejected non-empty lines", "optional": false}
                    ]
                },
                "stderr": {
                    "mimeType": "application/jsonl",
                    "description": "One JSON object per rejected non-empty line",
                    "fields": [
                        {"name": "line", "type": "number", "description": "1‑based physical line number", "optional": false},
                        {"name": "field", "type": "string|null", "description": "Failing field/path when available", "optional": true},
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
            }
        ],
        "globalFlags": [
            {"name": "help", "short": "-h", "long": "--help", "description": "Print help message and exit", "action": "help"},
            {"name": "version", "short": "-V", "long": "--version", "description": "Print version and exit", "action": "version"},
            {"name": "help-json", "short": "-hj", "long": "--help-json", "description": "Print machine-readable CLI surface description and exit", "action": "helpJson"}
        ],
        "rootBehavior": {"action": "help", "exitCode": 0},
        "exitCodes": {
            "0": "Success (hotspots: JSON written, including empty entries; record: all events accepted; init: hook installed; dashboard: server shut down cleanly; hook: always — fail-open, never blocks the harness)",
            "1": "Hotspots: storage error. Record: one or more events rejected, or I/O error writing output. Init: I/O error. Dashboard: port in use or artifact read error.",
            "2": "Usage error; hotspots: missing/unsupported store; record: also fatal I/O error (unreadable file or store failure); init: unsupported harness, collision, or self-install refusal; dashboard: invalid flags."
        }
    });
    serde_json::to_string(&doc).unwrap_or_else(|_| "{}".into())
}

pub(crate) fn write_cli_surface(out: &mut impl Write) -> io::Result<()> {
    write!(out, "{}", cli_surface_doc())
}
