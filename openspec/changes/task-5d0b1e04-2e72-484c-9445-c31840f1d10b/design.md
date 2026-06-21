## Context

Task `5d0b1e04-2e72-484c-9445-c31840f1d10b` was opened as “Replace record JSONL store with SQLite writer,” but repository evidence and the archived change `task-f875d4d7-fa49-4973-bff3-322f45e515fc` show that the main migration is already implemented. `scryrs-core` already owns a SQLite `EventStore`, the live `scryrs-record-endpoint` spec already names `.scryrs/scryrs.db` as canonical persistence, and verification paths already read from SQLite. The remaining work is a follow-up hardening pass that closes the specific gaps still called out by the backlog acceptance criteria and validated room decisions.

## Goals

- Ensure accepted events for a single `scryrs record` invocation are committed through an explicit batch/transaction boundary before success is reported.
- Prove at the CLI integration level that `--stdin` and `--file <PATH>` both populate `.scryrs/scryrs.db`, rejected lines never create rows, and `.scryrs/events.jsonl` is not the canonical output.
- Prove the fatal datastore path still exits loudly and deterministically with no fake success summary.
- Bring plain help and stale documentation surfaces into alignment with the existing SQLite contract.

## Non-Goals

- Re-implement the already-landed SQLite schema, indexes, normalized columns, or canonical store path from the archived migration.
- Change the JSONL ingestion wire format, rejection object shape, accepted/rejected summary contract, or hook invocation modes.
- Add hotspot analysis, query APIs, hosted storage, legacy `.scryrs/events.jsonl` migration, or any alternate canonical write path.
- Redesign help-json or broaden docs churn beyond the stale surfaces identified by refinement evidence.

## Decisions

### D1. Treat this as a narrow follow-up, not a fresh migration

The archived change already covers the substantive JSONL-to-SQLite replacement. This change only closes the remaining quality, verification, and discovery-surface gaps still needed to satisfy Task `5d0b1e04-2e72-484c-9445-c31840f1d10b`.

### D2. Keep transaction ownership in `scryrs-core`

The explicit invocation-level commit boundary should be exposed through a core-owned batch insert API or equivalent core-owned transaction wrapper. `scryrs-cli` should continue to compose the store API rather than managing raw SQLite transactions directly.

### D3. Success is gated on the batch commit, not per-row autocommit

Accepted events from one record invocation must be inserted through one explicit transaction or equivalent batch boundary. The command may only report success after that commit succeeds. This closes the current autocommit-per-row gap without adding a broader new durability-pragmas contract for this task.

### D4. CLI integration tests must query the SQLite store directly

Record-mode tests should reopen `.scryrs/scryrs.db` and assert `trace_events` contents for:

- valid `--stdin` ingestion,
- valid `--file <PATH>` ingestion,
- mixed valid/invalid ingestion proving rejected lines create no rows,
- fatal datastore failure proving exit code `2`, deterministic diagnostics, and no success summary,
- no canonical `.scryrs/events.jsonl` output.

### D5. Discovery-surface fixes are in scope where evidence shows drift

Update the plain help exit-code text, its snapshot, the README exit-code/error-path surface, and roadmap references that still describe `.scryrs/events.jsonl` as the Phase 1 store. Leave already-correct surfaces unchanged.

## Risks

- Adding a core-owned batch API changes `EventStore` borrowing and commit flow; the implementation should stay minimal and avoid exposing raw connection state to the CLI.
- Simulating deterministic datastore failure at the CLI layer may require a controlled filesystem or preexisting-database fixture.
- Help and docs updates can drift again if source text and generated snapshots are not refreshed together.

## Conflict Resolution

- **Duplicate vs follow-up scope:** Resolved as a follow-up. The archived change already completed the substantive SQLite migration, so this task should not reopen schema or persistence design from scratch.
- **Docs scope disagreement:** Resolved in favor of updating the stale surfaces. Repository evidence shows the README error-path table omits store failure and `.devagent/docs/docs/roadmap.mdx` still names `.scryrs/events.jsonl` as the Phase 1 store.
- **Help snapshot disagreement:** Resolved as “not stale, but still wrong.” The plain help snapshot matches current source; both should be updated together so exit code `2` mentions store failure.

## Traceability

- Task `5d0b1e04-2e72-484c-9445-c31840f1d10b`
- Dossier `2026-06-21T07:45:30.894Z`
- Accepted decisions `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base `initial`
