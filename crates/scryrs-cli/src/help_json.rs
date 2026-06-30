use std::io::{self, Write};

use serde_json::json;

/// Version of the `--help-json` surface document format, independent of
/// `SCHEMA_VERSION` which governs command output envelopes.
const SURFACE_VERSION: &str = "0.13.0";

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
                "description": "Ingest JSONL trace events from stdin or file. Defaults to remote (live) transport, submitting a single batch to POST /v1/trace-events/batch. Identity resolves by precedence: CLI flags, then SCRYRS_REMOTE_* environment variables, then .scryrs/.env, then the scryrs.json `remote` section. When live config is unresolved, the command fails fast (exit 2) with guidance instead of silently using local mode. Use --mode local for the SQLite store (.scryrs/scryrs.db).",
                "modes": [
                    {"name": "stdin", "flag": "--stdin", "description": "Read JSONL events from stdin"},
                    {"name": "file", "flag": "--file", "value": "PATH", "description": "Read JSONL events from PATH"},
                    {"name": "mode", "flag": "--mode", "value": "MODE", "values": ["live", "local"], "default": "live", "description": "Transport mode: live (default, remote ingest) or local (SQLite)"}
                ],
                "transport": {
                    "local": {
                        "description": "Explicit opt-in (--mode local) — persists accepted events to .scryrs/scryrs.db via the canonical EventStore.",
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
                        "description": "Default transport (live) — resolves an ingest URL from flags, env, .scryrs/.env, or scryrs.json `remote.ingest_url`. Skips .scryrs/scryrs.db entirely. Default timeout 3000 ms.",
                        "configPrecedence": ["1. CLI flags", "2. Environment variables (SCRYRS_REMOTE_*)", "3. .scryrs/.env", "4. scryrs.json `remote` section", "5. Git remote origin URL (repository_id fallback only)"],
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
                "description": "Install the scryrs trace hook for a supported agent harness (hook only). Idempotent and config-free: it never reads or writes scryrs.json or the .scryrs/ scaffold, and cannot fail on missing ingest config. Configure trace transport separately with `scryrs setup <mode>`.",
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
                    "description": "Hook-focused next-step instructions on stdout (confirms the hook was installed and directs the operator to run `scryrs setup <mode>` and reload their agent harness). Errors on stderr. Does not print a remote ingest URL or `scryrs up` guidance."
                }
            },
            {
                "name": "setup",
                "description": "Configure local or live trace transport. The only command that writes scryrs.json remote and the .scryrs/ config scaffold; independent of `init` (no hook is installed or required). `mode` is a required positional (local or live).",
                "arguments": [
                    {
                        "name": "mode",
                        "type": "string",
                        "required": true,
                        "values": ["local", "live"],
                        "description": "Transport mode: local (SQLite store under .scryrs/) or live (remote ingest via scryrs.json remote)"
                    },
                    {
                        "name": "ingest-url",
                        "flag": "--ingest-url",
                        "type": "string",
                        "description": "Live-mode remote ingest URL (overrides .scryrs/.env SCRYRS_REMOTE_INGEST_URL; resolved from env/.scryrs/.env/scryrs.json when omitted)"
                    },
                    {
                        "name": "workspace-id",
                        "flag": "--workspace-id",
                        "type": "string",
                        "description": "Live-mode workspace identity (overrides .scryrs/.env SCRYRS_WORKSPACE_ID)"
                    },
                    {
                        "name": "agent-id",
                        "flag": "--agent-id",
                        "type": "string",
                        "description": "Optional live-mode agent identity override (autogenerated per container from the hostname when omitted; not written to committed config)"
                    },
                    {
                        "name": "repository-id",
                        "flag": "--repository-id",
                        "type": "string",
                        "description": "Optional live-mode repository identity override (derived from Git remote origin when omitted; not written to committed config)"
                    },
                    {
                        "name": "with-compose",
                        "flag": "--with-compose",
                        "type": "boolean",
                        "default": false,
                        "description": "Live-mode opt-in: scaffold the self-hosted .scryrs/compose.yml stack plus an overrides-only .scryrs/.env (requires a docker_network) for `scryrs up`"
                    },
                    {
                        "name": "docker-network",
                        "flag": "--docker-network",
                        "type": "string",
                        "description": "External Docker network name for the --with-compose opt-in (overrides .scryrs/.env SCRYRS_DOCKER_NETWORK); not required by core setup live"
                    },
                    {
                        "name": "no-interactive",
                        "flag": "--no-interactive",
                        "type": "boolean",
                        "default": false,
                        "description": "Disable live-setup prompts; missing live config fails fast (exit 2) instead of starting the TTY-only wizard"
                    }
                ],
                "output": {
                    "mimeType": "text/plain",
                    "description": "Next-step instructions on stdout. Errors on stderr. setup local scaffolds .scryrs/scryrs.db + .scryrs/.gitignore (never touches scryrs.json). setup live create-or-merges the committed scryrs.json remote section (ingest_url + workspace_id required; repository_id/agent_id resolved at runtime, never committed). The compose opt-in (--with-compose) additionally scaffolds .scryrs/compose.yml + an overrides-only .scryrs/.env and writes remote.docker_network. Missing required live config starts a TTY-only wizard unless non-interactive (no TTY or --no-interactive), which fails fast."
                }
            },
            {
                "name": "up",
                "description": "Start the workspace-managed live-server Compose stack from .scryrs/compose.yml, resolving the external network from scryrs.json remote.docker_network (override via SCRYRS_DOCKER_NETWORK or .scryrs/.env)",
                "output": {
                    "mimeType": "text/plain",
                    "description": "Docker compose stdout on success; deterministic scaffold and network errors on stderr."
                },
                "exitCodes": {
                    "0": "Workspace-managed compose stack started successfully",
                    "1": "Docker invocation or docker compose runtime failure",
                    "2": "Missing scaffold files, missing external network, or unexpected arguments"
                }
            },
            {
                "name": "doctor",
                "description": "Run the installation and readiness diagnostic command",
                "flags": [
                    {
                        "name": "json",
                        "long": "--json",
                        "type": "boolean",
                        "default": false,
                        "description": "Emit machine-readable JSON using the same diagnostic categories as the default human-readable output"
                    }
                ],
                "output": {
                    "mimeType": "text/plain or application/json",
                    "description": "Human-readable summary by default, or JSON with binary version, command surface / feature availability, resolved mode, local store status, hook status, live server reachability when configured, and docs links when --json is used."
                },
                "exitCodes": {
                    "0": "Success with only ok/warn findings",
                    "1": "Output write failure",
                    "2": "One or more structural error findings"
                }
            },
            {
                "name": "propose",
                "description": "Generate reviewable knowledge proposals from hotspot and graph evidence",
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
                    "description": "Validated ProposalDocument JSON files written to .scryrs/proposals/. Count written to stdout. Errors on stderr."
                }
            },
            {
                "name": "proposals",
                "description": "Review proposal inbox artifacts without mutating source-of-truth surfaces",
                "subcommands": [
                    {
                        "name": "list",
                        "description": "Emit deterministic JSON rows for pending, accepted, and rejected proposal states",
                        "arguments": [
                            {
                                "name": "PATH",
                                "type": "string",
                                "required": true,
                                "description": "Path to the repository root directory"
                            }
                        ],
                        "flags": [
                            {
                                "name": "state",
                                "long": "--state",
                                "type": "string",
                                "default": "all",
                                "values": ["pending", "accepted", "rejected", "all"],
                                "description": "Optional proposal state filter"
                            }
                        ],
                        "output": {
                            "mimeType": "application/json",
                            "fields": [
                                {"name": "proposalId", "type": "string", "description": "Deterministic proposal identifier", "optional": false},
                                {"name": "title", "type": "string", "description": "Reviewer-facing proposal title", "optional": false},
                                {"name": "targetType", "type": "string", "description": "Proposal target kind", "optional": false},
                                {"name": "createdAt", "type": "string", "description": "RFC 3339 proposal creation timestamp", "optional": false},
                                {"name": "state", "type": "string", "description": "Current review state: pending, accepted, or rejected", "optional": false}
                            ]
                        }
                    },
                    {
                        "name": "accept",
                        "description": "Write a deterministic accepted ProposalReviewDecision under .scryrs/accepted/",
                        "arguments": [
                            {"name": "PATH", "type": "string", "required": true, "description": "Path to the repository root directory"},
                            {"name": "ID", "type": "string", "required": true, "description": "Proposal identifier to review"}
                        ],
                        "flags": [
                            {"name": "reviewer", "long": "--reviewer", "type": "string", "required": true, "description": "Reviewer identity"},
                            {"name": "rationale", "long": "--rationale", "type": "string", "required": true, "description": "Non-empty review rationale"},
                            {"name": "decided-at", "long": "--decided-at", "type": "string", "required": true, "description": "Explicit RFC 3339 review timestamp"}
                        ],
                        "output": {
                            "mimeType": "none",
                            "description": "Writes .scryrs/accepted/{proposalId}.json on success and preserves .scryrs/proposals/{proposalId}.json unchanged."
                        }
                    },
                    {
                        "name": "reject",
                        "description": "Write a deterministic rejected ProposalReviewDecision under .scryrs/rejected/",
                        "arguments": [
                            {"name": "PATH", "type": "string", "required": true, "description": "Path to the repository root directory"},
                            {"name": "ID", "type": "string", "required": true, "description": "Proposal identifier to review"}
                        ],
                        "flags": [
                            {"name": "reviewer", "long": "--reviewer", "type": "string", "required": true, "description": "Reviewer identity"},
                            {"name": "rationale", "long": "--rationale", "type": "string", "required": true, "description": "Non-empty review rationale"},
                            {"name": "decided-at", "long": "--decided-at", "type": "string", "required": true, "description": "Explicit RFC 3339 review timestamp"}
                        ],
                        "output": {
                            "mimeType": "none",
                            "description": "Writes .scryrs/rejected/{proposalId}.json on success and preserves .scryrs/proposals/{proposalId}.json unchanged."
                        }
                    }
                ]
            },
            {
                "name": "dashboard",
                "description": "Start dashboard server and open the browser dashboard. Live is the default source mode (proxies a scryrs server); use --mode local to read local .scryrs artifacts. Live targets resolve from flags, then env, then .scryrs/.env, then scryrs.json `remote`; unresolved live config fails fast (exit 2) with guidance.",
                "flags": [
                    {"name": "mode", "long": "--mode", "type": "string", "values": ["live", "local"], "default": "live", "description": "Source mode: live (default) or local"},
                    {"name": "port", "short": "-p", "long": "--port", "type": "number", "default": 8080, "description": "TCP port to bind"},
                    {"name": "bind", "short": "-b", "long": "--bind", "type": "string", "default": "127.0.0.1", "description": "Bind address"},
                    {"name": "server-url", "long": "--server-url", "type": "string", "description": "Live-mode scryrs server base URL (overrides .scryrs/.env SCRYRS_REMOTE_INGEST_URL)"},
                    {"name": "repository-id", "long": "--repository-id", "type": "string", "description": "Live-mode repository identity (overrides .scryrs/.env SCRYRS_REPOSITORY_ID)"},
                    {"name": "no-open", "long": "--no-open", "type": "boolean", "default": false, "description": "Do not open browser automatically"},
                    {"name": "dev", "long": "--dev", "type": "boolean", "default": false, "description": "Serve from filesystem instead of embedded assets"}
                ],
                "output": {
                    "mimeType": "text/html",
                    "description": "Vue.js SPA served over HTTP. REST API at GET /api/meta, GET /api/hotspots, GET /api/signals (live mode only), GET /api/sessions (local mode only), GET /api/sessions/:sessionId (local mode only), GET /api/events (local mode only)."
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
            },
            {
                "name": "route",
                "description": "Generate the route manifest from a knowledge graph artifact, or query the manifest for matching entries",
                "arguments": [
                    {
                        "name": "PATH",
                        "type": "string",
                        "required": true,
                        "description": "Path to the repository root directory"
                    }
                ],
                "subcommands": [
                    {
                        "name": "explain",
                        "description": "Query the route manifest for matching entries using case-insensitive substring matching",
                        "arguments": [
                            {
                                "name": "PATH",
                                "type": "string",
                                "required": true,
                                "description": "Path to the repository root directory"
                            },
                            {
                                "name": "query",
                                "flag": "--query",
                                "type": "string",
                                "required": true,
                                "description": "Query text for case-insensitive substring matching against label, subject, id, target, kind, and evidence_links[].subject"
                            }
                        ],
                        "matching": {
                            "algorithm": "case-insensitive substring match",
                            "fields": ["label", "subject", "id", "target", "kind", "evidence_links[].subject"],
                            "tiers": [
                                {"tier": 3, "description": "Exact string match"},
                                {"tier": 2, "description": "Prefix match"},
                                {"tier": 1, "description": "Substring match"}
                            ],
                            "tieBreak": "Manifest entry order (by id ascending) within each tier"
                        },
                        "output": {
                            "mimeType": "application/json",
                            "description": "Single-line RouteHintDocument JSON with schemaVersion and hints array. The reason field appends '; query match on <fields>' suffix. Zero matches produces a valid document with empty hints array."
                        },
                        "exitCodes": {
                            "0": "Success (including zero-match results)",
                            "1": "Serialization or stdout write failure",
                            "2": "Usage error, missing .scryrs/routes.json, malformed JSON, or schema version mismatch"
                        }
                    }
                ],
                "output": {
                    "mimeType": "application/json",
                    "description": "Single-line RouteManifestDocument JSON written to stdout. Also persisted to .scryrs/routes.json."
                },
                "routeHintOutput": {
                    "mimeType": "application/json",
                    "description": "Deterministic RouteHintDocument projection derived from the route manifest. Each route entry produces one RouteHintItem with identity, target, label, 1-based ordinal rank, evidence citations, and a template-derived reason. Rank is a deterministic ordinal derived from manifest entry sort order; relevance is deferred (None). Use `scryrs route explain <PATH> --query <TEXT>` to filter and rank hints by query match.",
                    "fields": [
                        {"name": "schemaVersion", "type": "string", "description": "Route hint schema version (always HINT_SCHEMA_VERSION = 1.0.0)", "optional": false},
                        {"name": "hints", "type": "array", "description": "Deterministically ordered array of RouteHintItem objects", "optional": false}
                    ],
                    "hintItemFields": [
                        {"name": "routeId", "type": "string", "description": "Source route entry id", "optional": false},
                        {"name": "target", "type": "string", "description": "Normalized load target", "optional": false},
                        {"name": "label", "type": "string", "description": "Human-readable label", "optional": false},
                        {"name": "rank", "type": "number", "description": "1-based ordinal rank from manifest entry sort order (deterministic ordinal, not final ranking)", "optional": false},
                        {"name": "relevance", "type": "number|null", "description": "Optional relevance score — deferred for future enhancement (always null in current version)", "optional": true},
                        {"name": "reason", "type": "string", "description": "Deterministic template reason citing route entry identity and evidence count", "optional": false},
                        {"name": "evidence", "type": "array", "description": "Evidence provenance links copied from source route entry", "optional": true}
                    ],
                    "example": {
                        "schemaVersion": "1.0.0",
                        "hints": [
                            {
                                "routeId": "file:src/main.rs",
                                "target": "file:src/main.rs",
                                "label": "src/main.rs",
                                "rank": 1,
                                "reason": "Route 'src/main.rs' (file:src/main.rs): 2 evidence link(s), subject kind file",
                                "evidence": [
                                    {
                                        "sourceKind": "local_trace_row",
                                        "subject": "src/main.rs",
                                        "rowIds": [1, 2]
                                    }
                                ]
                            }
                        ]
                    },
                    "rankingPolicy": "Rank is a deterministic 1-based ordinal derived from manifest entry sort order (by id ascending). Relevance is deferred (None) and does not represent a frozen long-term ranking formula. Both fields are explicitly documented as deferred or ordinal."
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
            "0": "Success (hotspots: JSON written, including empty entries; record local: all events accepted; record remote: no rejections or failures; init: hook installed; up: workspace-managed compose stack started; doctor: only ok/warn findings; propose/proposals: artifacts written or listed successfully; dashboard: server shut down cleanly; server: server shut down cleanly; hook: always — fail-open, never blocks the harness)",
            "1": "Hotspots: storage error. Record: one or more events rejected (local or server), or I/O error writing output. Init: I/O error. Up: docker invocation failure. Doctor: output write failure. Proposals: serialization or filesystem write failure. Dashboard: port in use or artifact read error. Server: port in use or store error.",
            "2": "Usage error; hotspots: missing/unsupported store; record: also fatal I/O error (unreadable file, store failure, missing remote identity, transport timeout, connection failure, non-2xx response, malformed response); init: unsupported harness, collision, or self-install refusal; setup: unknown/missing mode, source-checkout refusal (live), or missing/invalid/conflicting live configuration; up: missing scaffold files, missing external network, or unexpected arguments; doctor: one or more structural error findings; proposals: invalid filter, invalid proposal/review document, unknown proposal ID, or conflicting terminal review state; dashboard: invalid flags or partial live-mode configuration; server: invalid flags or bind failure."
        }
    });
    serde_json::to_string(&doc).unwrap_or_else(|_| "{}".into())
}

pub(crate) fn write_cli_surface(out: &mut impl Write) -> io::Result<()> {
    write!(out, "{}", cli_surface_doc())
}
