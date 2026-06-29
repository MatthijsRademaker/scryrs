# Live Hotspots

Live hotspots are the multi-agent counterpart to local batch hotspots: instead of each agent session writing to its own isolated `.scryrs/scryrs.db`, a central `scryrs server` receives trace events from every agent, maintains shared cumulative hotspot state, and exposes current rankings and threshold-crossing signals as live query and streaming APIs. This page explains what live mode is for, how it works end to end, when to choose it over local batch mode, and how to interpret the live API response fields. For the full endpoint tables, JSON schemas, exit codes, and allowed values, see the [CLI v0 Contract](./cli-v0-contract.md).

## What Problem Live Mode Solves

Local batch hotspots work well for a single agent on one machine: `scryrs hotspots <PATH>` reads `.scryrs/scryrs.db`, runs deterministic scoring, and emits a `HotspotsReport`. But when multiple agents run in parallel — across CI workers, development containers, or harness instances — each agent's `.scryrs/scryrs.db` is an isolated island. To see the full picture, someone has to run `scryrs hotspots` on every machine, manually aggregate the reports, and deal with the fact that event identity and duplicate detection have no shared foundation.

Live mode replaces the per-machine SQLite islands with a single server-owned source of truth. Every agent in the team sends trace events to one server process, and the server's job is to:

- **Accept** valid trace events into a shared, server-owned SQLite store.
- **Deduplicate** event submissions so retries and reconnections do not inflate scores.
- **Maintain** cumulative accumulator state updated atomically on each accepted event.
- **Serve** live hotspot rankings on demand via a REST query endpoint.
- **Stream** threshold-crossing signals via Server-Sent Events so clients can react immediately rather than polling artifact files.

## What Live Mode Achieves

| Capability | What it means |
|------------|---------------|
| **Central ingest** | All agent instances POST trace events to `scryrs server` instead of writing to their own `.scryrs/scryrs.db`. The server is the single writer of record. |
| **Idempotent shared scoring** | The server deduplicates events by composite key `(repository_id, workspace_id, agent_id, producer_event_id)`. Resubmitting the same event is acknowledged as idempotent — scores never double-count. |
| **Cumulative live state** | The server maintains `hotspot_accumulators` that update in the same transaction as event inserts. Scores, counts, session membership, and timestamps evolve incrementally as events arrive from any agent. |
| **Live query API** | `GET /v1/repositories/{repository_id}/hotspots` returns a `LiveHotspotsResponse` with current rankings. Optional `?session_id` recomputes session-scoped rankings from raw events. |
| **Signal streaming** | `GET /v1/repositories/{repository_id}/signals` delivers `HotspotSignal` records via SSE when a subject's cumulative score crosses the configured threshold. Clients can replay from any cursor with `?after=<signal_id>`. Late-joining subscribers never miss signals. |

## End-to-End Live Workflow

The live hotspot pipeline is a continuous loop, not a batch job:

```
Agent hooks capture TraceEvent records
            ↓
scryrs record submits remote batch (wrapped in ServerIngestEnvelope)
            ↓
POST /v1/trace-events/batch
            ↓
scryrs server validates, deduplicates, and persists
            ↓
New subject-bearing events update cumulative accumulator state atomically
            ↓
Client reads: GET /v1/repositories/{id}/hotspots  (live rankings)
Client streams: GET /v1/repositories/{id}/signals  (SSE threshold-crossing signals)
```

1. **Capture** — Harness hooks continue to format `TraceEvent` records exactly as they do in local mode. The hook contract and event schema are identical; remote mode adds a transport wrapper (`ServerIngestEnvelope`) without changing the inner event shape. See the [Trace Hook Contract](./trace-hook-contract.md) for hook configuration.

2. **Submit** — `scryrs record` wraps events in a `ServerIngestEnvelope` with stable repository identity, workspace identity, agent identifier, and per-event producer IDs. The batch is POSTed to `POST /v1/trace-events/batch`.

3. **Ingest** — The server validates the envelope and each inner `TraceEvent`. Valid subject-bearing events are accepted and persisted to the server-owned SQLite store with a server-side `received_at` timestamp. Rejected events (schema-invalid, malformed timestamps) are reported per-item without halting the batch.

4. **Deduplicate** — The server applies first-writer-wins deduplication on the composite key `(repository_id, workspace_id, agent_id, producer_event_id)`. Events already stored are acknowledged as idempotent and do not change accumulator state.

5. **Accumulate** — Each newly accepted subject-bearing event updates the matching `hotspot_accumulators` row in the same SQLite transaction. The accumulator carries cumulative score, per-event-type counts, per-outcome counts, distinct-session state, and `first_seen`/`last_seen` timestamps. Scoring uses the same deterministic weight table as local batch `scryrs hotspots` — `per_event_contribution()` is the shared contract.

6. **Signal** — When a subject's cumulative score crosses from below to at-or-above the configured `signal_threshold` (default `10`), the server commits an append-only `HotspotSignal` row. The signal carries subject identity, new cumulative score, score delta, window tag, threshold value, ordered evidence row IDs, and a creation timestamp. Additional events above the threshold do not create duplicate crossing signals under the cumulative window model.

7. **Query and stream** — Clients query `GET /v1/repositories/{repository_id}/hotspots` for current rankings or connect to `GET /v1/repositories/{repository_id}/signals` for a live SSE stream. The signal stream replays persisted signals (in `id ASC` order, from the requested cursor forward), then tails new signals as they are committed. A 15-second keep-alive heartbeat prevents the connection from timing out during quiet periods.

## Live Mode vs Local Batch Hotspots

Live mode and local batch mode are **exclusive deployment choices**, not additive layers. A repository operates in one mode at a time, and the server-owned state does not merge with any pre-existing local `.scryrs/scryrs.db`.

| Dimension | Local batch (`scryrs hotspots`) | Live server (`scryrs server`) |
|-----------|----------------------------------|-------------------------------|
| **Source of truth** | `.scryrs/scryrs.db` on the local machine | Server-owned SQLite (`scryrs/server.db`) |
| **Ingest target** | `scryrs record --stdin` writes locally | `POST /v1/trace-events/batch` |
| **State model** | Deterministic batch re-scoring of all stored events on every `scryrs hotspots` invocation | Incremental cumulative accumulators updated per accepted event |
| **Deduplication** | Not applicable (single-writer filesystem) | First-writer-wins on `(repo, workspace, agent, producer_event_id)` |
| **Query** | CLI invocation reads SQLite, emits `HotspotsReport` to stdout | `GET /v1/repositories/{id}/hotspots` returns `LiveHotspotsResponse` |
| **Session scoping** | Not supported (full-store batch only) | Optional `?session_id` recomputes rankings from matching raw events |
| **Signals** | Not applicable (batch output only) | SSE stream of `HotspotSignal` records with `?after=` cursor replay |
| **Multi-agent sharing** | Manual — each machine runs its own `scryrs hotspots` | Automatic — all agents feed one server; one source of truth |

**When to use local batch:** You are a single developer or a single agent session on one machine. You want a self-contained, zero-network hotspot report. `scryrs hotspots .` gives you everything you need.

**When to use live mode:** You have multiple agent instances — CI pipelines, parallel development containers, or a team sharing a harness workspace. You want shared, continuously updated hotspot state without coordinating artifact files across machines. You want instant threshold-crossing signals instead of periodic polling.

## Interpreting Live Hotspot Fields

This section explains what each field in the live API responses means in domain terms. For the complete JSON schemas, endpoint tables, allowed values, and exit codes, see the [CLI v0 Contract](./cli-v0-contract.md).

### LiveHotspotsResponse Fields

The `LiveHotspotsResponse` is the live equivalent of the local batch `HotspotsReport`. It exists in the server's response domain and omits filesystem-specific fields (`repositoryPath`, `storePath`) that have no meaning in a multi-tenant server context.

**`entries`** — Ranked array of `HotspotEntry` items, sorted by the same six-key tie-break chain used in local batch mode: `score DESC → sessionCount DESC → lastSeen DESC → subjectKind ASC → subject ASC → firstEventId ASC`. Each entry contains:

- **`score`** — The cumulative agent effort for this subject across all accepted events from all agents. Uses the same deterministic weight table as local batch scoring: `FileOpened`/`CommandExecuted` weight 1, `SearchRun`/`SymbolInspected`/`DocRetrieved` weight 2, `EditMade` weight 3, `FailedLookup` weight 4 (plus a +2 failure bonus per failed event). Higher scores reflect more cumulative agent attention, not code quality.

- **`sessionCount`** — In cumulative (default) queries: the number of distinct agent sessions that touched this subject. High breadth means the subject is a cross-cutting concern across many independent sessions. In session-scoped queries (`?session_id=<id>`): always `1`, since the query is scoped to a single session.

- **`cursor`** — An opaque string field reserved for future use in cursor-based pagination of live hotspot results. Currently empty; clients should not parse or depend on its value.

- **`evidence.rowIds`** — An ordered list of `server_trace_events` row IDs that contributed to this hotspot entry's accumulator. These server-side row IDs provide traceability back to specific ingestion events and are ordered by `timestamp ASC, id ASC`.

- **`counts.eventType`** — Breakdown of event types contributing to the subject's score. Same semantics as local batch: each key is an event type name, each value is the per-subject occurrence count.

- **`counts.outcome`** — Split between `"success"` and `"failure"` outcomes for this subject's events. A high failure ratio signals a fragile or error-prone area — the same diagnostic value as local batch hotspots.

**`schemaVersion`** — The live hotspot schema version (`"1.0.0"`), independent of both the `TraceEvent` wire schema and the local `HotspotsReport` schema version.

**`repositoryId`** — The stable repository identifier derived from the Git remote origin URL (protocol-agnostic, lowercased, trailing-slash-stripped), or an explicit configured ID. Two clones of the same repository on different machines see the same live state.

**`generatedAt`** — Server wall-clock timestamp when this response was computed, in RFC 3339 format.

### HotspotSignal Fields

A `HotspotSignal` is emitted as an SSE event when a subject's cumulative score crosses the configured threshold for the first time. It tells you "this subject just became actionable."

- **`score`** — The subject's new cumulative score after the triggering event was applied. Reflects all agent attention to date, not just the triggering contribution.

- **`delta`** — The score contribution of the triggering event that caused the threshold crossing. Minimum value is `1`. This is the amount by which the score changed — it is NOT the total score.

- **`threshold`** — The signal threshold in effect when the crossing occurred. Defaults to `10`. A signal fires when the cumulative score transitions from below this value to at-or-above it. Once a subject is above the threshold, additional events increase the score but do not create new crossing signals under the cumulative window model.

- **`window`** — The accumulator window model. Currently always `"cumulative"`. Future additive window models (e.g., rolling windows) would carry distinct window tags.

- **`evidenceRowIds`** — Ordered list of `server_trace_events` row IDs that contributed to the subject's accumulator at the time the signal fired. Traceable back to individual agent actions for detailed investigation.

- **`subjectKind`** and **`subject`** — The subject identity, matching the same `(kind, subject)` grouping key used across local and live hotspots. Kinds: `file`, `search`, `symbol`, `command`, `document`.

- **`createdAt`** — Server timestamp (RFC 3339) when the signal was persisted.

## Getting Started

Start the server:

```bash
scryrs server --bind 0.0.0.0 --port 8081 --store /data/scryrs/server.db
```

This starts the long-lived HTTP server with all three REST endpoints. The startup message prints the listen address and store path to stderr.

To configure hooks for remote mode, all nine event families and the `TraceEvent` schema remain identical to local mode. Hooks continue to emit the same `TraceEvent` records, and `scryrs record` handles the transport wrapper automatically when remote ingest is configured. See the [CLI v0 Contract](./cli-v0-contract.md) for the complete endpoint surface and the [Trace Hook Contract](./trace-hook-contract.md) appendix for remote ingestion identity field semantics and the `ServerIngestEnvelope` transport contract.

## Live Dashboard Mode

The dashboard now has a matching **live read path**. Start it with both live flags:

```bash
scryrs dashboard --server-url http://127.0.0.1:8081 --repository-id repo-a
```

Live dashboard mode keeps the browser on same-origin `/api/*` calls and lets the dashboard backend proxy the live server contract:

- `GET /api/meta` reports `mode: "live"` and the configured `repositoryId`.
- `GET /api/hotspots` proxies `GET /v1/repositories/{repository_id}/hotspots?window=cumulative` and preserves the upstream `cursor`.
- `GET /api/signals?after=<id>` proxies the server SSE endpoint and streams replayed plus live `HotspotSignal` events without buffering the full upstream response.

The browser owns reconnect behavior for the current page lifecycle. On first open, the Signals view connects to `/api/signals?after=0`. After a disconnect it reconnects with the last seen SSE id, for example `/api/signals?after=57`, and ignores replay duplicates that the server legitimately re-sends on resume. A full page refresh starts over from `after=0`.

Local and live dashboard modes stay deliberately separate:

| Concern | Local dashboard | Live dashboard |
| --- | --- | --- |
| Hotspot source | `.scryrs/hotspots.json` | `scryrs server` cumulative query |
| Session/Event views | Available | Hidden from navigation; direct URLs show an unavailable explanation |
| Signals view | Unavailable | Available with explicit connection-state UI |
| Subject rendering | Repo-relative file shortening when possible | Raw server subject strings, no implied local artifact path |
| Fallback behavior | Reads local artifacts only | No local fallback or local/live merge |

## Related Pages

- [Hotspots](./hotspots.md) — domain-oriented explanation of local batch hotspots, scoring, and interpretation
- [CLI v0 Contract](./cli-v0-contract.md) — complete endpoint tables, JSON schemas, exit codes, and the `scryrs server` invocation contract
- [Trace Hook Contract](./trace-hook-contract.md) — how harness hooks capture `TraceEvent` records, plus the remote ingestion appendix with `ServerIngestEnvelope` identity semantics
- [Architecture](./architecture.mdx) — crate topology including `scryrs-server`'s live accumulator and signal streaming design
- [Product Roadmap](./roadmap.mdx) — Phase 4 delivery scope and accepted limitations for live hotspot server features
