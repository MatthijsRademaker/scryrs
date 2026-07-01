# Hotspots

Hotspots expose the context your AI agents keep rediscovering across sessions — so you can promote that repeated effort into durable documentation, investigate fragile areas, and stop paying the same discovery cost session after session.

## What a Hotspot Represents

A **hotspot** is a subject (a file, search query, symbol, command, or document reference) that appears across multiple agent sessions, accumulating repeated context-churn effort. When agents open the same file, search the same term, inspect the same symbol, or fail to find the same concept over and over, scryrs detects that repetition and produces a ranked `HotspotsReport`.

The core insight: **repeated agent effort around a subject indicates knowledge that should be documented or investigated**, not that the subject is inherently bad. A high hotspot score means agents are spending effort there — the right response might be writing architecture docs, creating a debugging playbook, fixing a fragile API, or recording a design decision.

**Score is a proxy for context-churn effort, not for code quality, business value, or correctness.** A frequently opened utility file with simple, correct code may score higher than a buggy, undocumented core module that nobody touches because everyone avoids it. Hotspots answer "where are agents spending effort?", not "what is good or bad?"

## How Hotspots Are Computed: the Local Batch Workflow

`scryrs hotspots <PATH>` defaults to the local deterministic batch pipeline:

```
Agent hook captures TraceEvent records
        ↓
scryrs record persists to .scryrs/scryrs.db (SQLite)
        ↓
scryrs hotspots <PATH> reads the store and runs deterministic scoring
        ↓
HotspotsReport emitted to stdout and written to .scryrs/hotspots.json
```

Every `scryrs hotspots <PATH>` invocation produces the same output given the same `.scryrs/scryrs.db` — there is no randomness, no model inference, and no time-dependent behavior in scoring. The only wall-clock dependency is the `generatedAt` timestamp in the report envelope.

For the full output contract — including the envelope schema, exit codes, store paths, and artifact file behavior — see the [CLI v0 Contract](./cli-v0-contract.md).

## Explicit Live Artifact Export

When hotspot state lives on `scryrs server`, `scryrs hotspots` can materialize that server-owned ranking into the same `.scryrs/hotspots.json` artifact shape consumed by `scryrs graph`, `scryrs route`, and `scryrs propose`:

```text
scryrs hotspots <PATH> --mode live
        ↓
resolve server-url + repository-id
(flags → env → .scryrs/.env → scryrs.json remote)
        ↓
GET /v1/repositories/{repository_id}/hotspots?window=cumulative
        ↓
HotspotsReport emitted to stdout and written to .scryrs/hotspots.json
```

This live export path is deliberately strict:

- It **never opens** `<PATH>/.scryrs/scryrs.db`.
- It **never merges** local SQLite-only subjects into the exported entries.
- It sets `storePath` to `live:<query_url>` so the artifact provenance is explicit.
- It copies `generatedAt` from the server response and derives `runMetadata` from the returned live entries.
- It writes `.scryrs/hotspots.json` atomically, so fetch or validation failures leave any existing artifact untouched.

The purpose is compatibility, not a new downstream protocol: live mode materializes the existing `HotspotsReport` envelope so the rest of the artifact pipeline keeps working unchanged.

## Interpreting Hotspot Fields

Each entry in the `entries` array carries the following fields. Understanding what each one signals is the key to acting on a hotspot report.

### subject

The identity of the hotspot — a file path, search query, symbol name, command string, or document reference. This is the thing agents keep interacting with.

### subjectKind

The category of the subject, always one of:

| Kind | What it captures |
|------|-----------------|
| `file` | A file path agents open or edit (`src/auth/handlers.rs`) |
| `search` | A search query agents repeat (`error handling pattern`) |
| `symbol` | A symbol agents inspect or fail to find (`Authenticator::verify`) |
| `command` | A shell command agents execute (`cargo test -p auth`) |
| `document` | A document reference agents retrieve (`api/auth-flow.md`) |

A subject and kind pair is the grouping key: `("file", "src/main.rs")` and `("search", "src/main.rs")` are separate hotspots because they represent different types of agent attention.

### score

The sum of weighted event contributions for the subject, plus failure penalties. Higher scores reflect more cumulative agent effort — more frequent interactions and more expensive interaction types.

Each event type carries a base weight reflecting its relative context cost, and failure events add a fixed penalty on top:

| Event type | Base weight | Notes |
|-----------|-------------|-------|
| `FileOpened` | 1 | Low cost — reading a file is routine |
| `CommandExecuted` | 1 | Routine command execution |
| `SearchRun` | 2 | Higher cost — re-searching implies missing knowledge |
| `SymbolInspected` | 2 | Higher cost — symbol lookup implies unfamiliarity |
| `DocRetrieved` | 2 | Doc retrieval implies knowledge gaps |
| `EditMade` | 3 | Highest base cost — edits represent substantive work |
| `FailedLookup` | 4 | Highest base weight — failed lookups signal the largest knowledge gap |

**Failure bonus:** Every event with an `Outcome::Failure` adds +2 on top of its base weight. A `FailedLookup` scores 4 + 2 = 6; a failed `EditMade` scores 3 + 2 = 5.

For the complete weight table and scoring contract, see the [CLI v0 Contract](./cli-v0-contract.md#scoring-dimensions).

### counts.eventType

A breakdown of which agent activity types contributed to the subject's score. Each key is an event type name (`"FileOpened"`, `"EditMade"`, `"FailedLookup"`, etc.) and each value is how many times that event type occurred for this subject. Only non-zero counts appear.

Use this to understand the *shape* of agent attention: a file with high `EditMade` counts is being actively changed; a subject with high `FailedLookup` counts represents a persistent knowledge gap; a subject with many `SearchRun` events suggests a concept agents struggle to locate.

### counts.outcome

The split between `"success"` and `"failure"` outcomes for this subject's events. A high ratio of failures to successes signals a fragile or error-prone area. A subject with exclusively failure events (especially `FailedLookup`s) indicates something agents repeatedly try and fail to find — a strong signal that documentation is missing.

### sessionCount

The number of distinct agent sessions in which this subject appeared. High session breadth means the subject is a **cross-cutting concern**, not a one-session anomaly. A file opened in 15 different sessions is a systemic churn point; a file opened 15 times in one session may just be the session's focus area.

Session count is a critical differentiator when scores are similar. A subject with score 20 across 10 sessions represents a wider, more systemic pattern than a subject with score 20 from a single intense session.

### evidence.rowIds

An ordered list of SQLite row IDs that reference the specific `trace_events` rows contributing to this hotspot entry. These row IDs can be traced back to individual agent actions for detailed investigation — use the dashboard or query `.scryrs/scryrs.db` directly to inspect the full event details behind a particular hotspot.

Ranking within the `entries` array is deterministic and uses a six-key tie-break chain: `score DESC → sessionCount DESC → lastSeen DESC → subjectKind ASC → subject ASC → firstEventId ASC`. Given the same input data, the ranking is always identical.

## What to Do with a Hotspot Report

Hotspot output is actionable. Connect the signal components to concrete decisions:

| Signal | What it suggests | Action |
|--------|-----------------|--------|
| High-scored files | Agents spend significant effort navigating or editing these files | Write or update architecture documentation for these files and their modules |
| Repeated `FailedLookup` events | Agents repeatedly search for something that doesn't exist | Create the missing documentation, add a glossary entry, or record a design decision explaining the concept |
| High `sessionCount` across many subjects | Cross-cutting concerns affecting many independent sessions | Write a design decision record (ADR) capturing the systemic pattern |
| High failure density in `counts.outcome` | Fragile area where agent interactions frequently fail | Investigate the underlying cause — flaky tests, unclear API, missing error documentation |
| High `SearchRun` counts for a query | Concept that agents cannot locate efficiently | Add a route or index entry, improve module-level docs, or create a navigation guide |
| Specific `evidence.rowIds` | Traceability back to exact agent actions | Use the dashboard or SQLite to inspect individual events and understand what the agent was doing |

## What Hotspots Do NOT Tell You

To avoid misinterpreting the report, be clear about what hotspot scores are not:

- **Not a code quality metric.** A high-scored file may be well-written but frequently referenced. A low-scored file may be buggy but ignored.
- **Not a business value indicator.** Score reflects agent effort, not revenue impact or user-facing importance.
- **Not a correctness measure.** Correct code that agents reference often scores high; incorrect code that nobody touches scores zero.
- **Not a priority list.** Use hotspot output as evidence for documentation investments, not as the sole input to roadmap ordering.

**Related: Live Hotspot Server** — The live hotspot server (`scryrs server`) is a separate deployment mode that provides central ingestion, shared live state, signal streaming, and explicit `scryrs hotspots --mode live` export for multi-agent teams. See [Live Hotspots](./live-hotspots.md) for the domain narrative and mode comparison.

## Related Pages

- [CLI v0 Contract](./cli-v0-contract.md) — output envelope, exit codes, weight table, and the `scryrs hotspots` invocation contract
- [Trace Hook Contract](./trace-hook-contract.md) — how harness hooks capture `TraceEvent` records that feed hotspot analysis
- [Architecture](./architecture.mdx) — crate topology including `scryrs-core` scoring and the `HotspotsReport` data flow
- [Vision & Goals](./vision.md) — product positioning and the "Observe → Detect → Promote → Route" product loop
- [Graph](./graph.md) — domain-oriented explanation of the knowledge graph and how evidence becomes routing context
