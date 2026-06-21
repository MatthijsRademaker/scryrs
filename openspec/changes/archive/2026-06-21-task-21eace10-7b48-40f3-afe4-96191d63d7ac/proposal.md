# Proposal: Hotspot Foundation 02 — Define hotspot output contract and SQLite ranking rules

## Why

Phase 2 of the scryrs roadmap requires real hotspot analysis over `.scryrs/scryrs.db`. The current public surface still emits only `{"schemaVersion","command","status":"placeholder"}` and the shared `Hotspot` type carries only `subject` plus `score`. The SQLite store and `TraceQuery` read model already provide deterministic, indexed evidence sufficient for ranking. The missing work is to freeze one versioned hotspot schema and one deterministic ranking/tie-break policy.

## What Changes

- **New `HotspotsReport` and `HotspotEntry` types** in `scryrs-types` replacing the current `Hotspot` struct with a full evidence-carrying schema including `subjectKind`, per-event-type counts, per-outcome counts, `sessionCount`, `firstSeen`/`lastSeen` timestamps, and SQLite row ID evidence references.
- **Independent `HOTSPOT_SCHEMA_VERSION`** constant ("1.0.0"), decoupled from trace event `SCHEMA_VERSION` ("0.1.0").
- **Deterministic scoring function** (`score_hotspots`) in `scryrs-core` that groups events by `(subject_kind, subject)`, applies a documented integer weight table per event type plus a failure bonus, and produces ranked entries with a six-key tie-break chain.
- **Real CLI hotspot output** — `scryrs hotspots <PATH>` opens the SQLite store via `TraceQuery`, scores subjects, and emits the `HotspotsReport` envelope to stdout and to `.scryrs/hotspots.json`.
- **Updated `--help-json` surface**, CLI help text, and README to reflect the new report schema instead of the placeholder fields.
- **Error handling** — explicit exit codes: exit 0 for valid store (populated or empty with `entries: []`), exit 2 for `MissingStore`/`UnsupportedStore`, exit 1 for `StorageError`.
- **Superseded OpenSpec contract** — the `scryrs-trace-query` spec's placeholder requirement is replaced by a new `hotspot-report` spec.
- **Updated downstream consumer** — `scryrs-curator::propose_from_hotspot` signature updated to accept the richer `HotspotEntry` type.

## Impact

**Affected crates:** `scryrs-types`, `scryrs-core`, `scryrs-cli`, `scryrs-curator`

**Affected specs:** New `hotspot-report` spec; `scryrs-trace-query` spec updated to remove placeholder requirement.

**Affected docs:** CLI help text, `--help-json` surface document, inline snapshot tests.

**Downstream consumers:** `scryrs-curator` API signature changes (must accept `HotspotEntry` instead of `Hotspot`). Future graph/proposal tasks can consume the stable contract directly.

**Backwards compatibility:** The `Hotspot` struct is replaced by `HotspotEntry` — this is a breaking type-level change within the workspace. No external API stability guarantee exists at this phase.

**Risk level:** Medium. Weight table is initial defaults with documented evolution support. Evidence row ID arrays are unbounded in v1 (mitigation: document as known risk with future capping path).