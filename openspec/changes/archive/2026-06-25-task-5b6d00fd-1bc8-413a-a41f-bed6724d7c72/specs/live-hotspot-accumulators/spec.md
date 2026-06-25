## ADDED Requirements

### Requirement: Accepted subject-bearing server events update cumulative hotspot state atomically

The server SHALL maintain cumulative live hotspot state in the server-owned SQLite store. For each newly accepted subject-bearing event, the server SHALL insert the event row and commit the matching cumulative accumulator mutation in the same SQLite transaction. The accumulator identity SHALL be `(repository_id, window, subject_kind, subject)` with `window = "cumulative"` for this task. The accumulator SHALL retain the deterministic aggregate fields needed to materialize cumulative hotspot state: score, per-event-type counts, per-outcome counts, distinct-session state, `first_seen`, and `last_seen`.

#### Scenario: First accepted subject-bearing event creates cumulative state

- **GIVEN** no cumulative accumulator row exists for `(repo-a, "cumulative", "file", "src/main.rs")`
- **WHEN** the server accepts and stores a subject-bearing event for that repository and subject
- **THEN** exactly one `server_trace_events` row is created
- **AND** exactly one matching cumulative accumulator row is created or updated in the same SQLite transaction
- **AND** the accumulator score, counts, session state, `first_seen`, and `last_seen` include that event

#### Scenario: Lifecycle and rejected events do not mutate live hotspot state

- **GIVEN** an ingest batch containing a lifecycle event or an event rejected during validation
- **WHEN** the batch is processed
- **THEN** no hotspot accumulator row is created or updated for that item
- **AND** no hotspot signal row is created for that item

#### Scenario: Duplicate replay does not change accumulator state

- **GIVEN** a subject-bearing event for `(repo-a, ws-1, pi, evt-001)` was already accepted
- **WHEN** the same composite key is submitted again
- **THEN** the replay is acknowledged as idempotent
- **AND** the matching accumulator score, counts, session state, and timestamps remain unchanged

### Requirement: Live accumulation reuses deterministic hotspot scoring semantics

The server SHALL compute live per-event contributions through a shared deterministic scoring API in `scryrs-core`, using the same event-type weight table and failure bonus as batch hotspot analysis. The live path SHALL NOT duplicate or reinterpret the weight table. When cumulative live state is materialized for comparison or read helpers, evidence ordering SHALL follow `timestamp ASC, id ASC` so cumulative rank and evidence semantics stay aligned where batch and live semantics overlap.

#### Scenario: Failure bonus matches batch scoring

- **GIVEN** an `EditMade` event with `Outcome::Failure`
- **WHEN** the event is applied to the live accumulator
- **THEN** the contribution is `3 + 2 = 5`
- **AND** that contribution matches the batch hotspot score for the same event

#### Scenario: Cumulative live scoring matches batch scoring for the same event set

- **GIVEN** the same subject-bearing event set is applied to live server accumulation and batch `score_hotspots`
- **WHEN** tests compare the cumulative outputs
- **THEN** subject scores match
- **AND** ranked order matches for the overlapping cumulative semantics

#### Scenario: Materialized evidence ordering matches chronological batch ordering

- **GIVEN** two accepted events for the same subject arrive out of timestamp order
- **WHEN** cumulative live evidence references are materialized for comparison
- **THEN** the evidence references are ordered by `timestamp ASC, id ASC`
- **AND** the same order is used for batch/live alignment assertions

### Requirement: Threshold crossings persist hotspot signal history separately from accumulator state

The server SHALL persist append-only `HotspotSignal` records separately from accumulator rows. A signal SHALL be emitted only when an accepted accumulator update changes a subject's cumulative score from below the configured threshold to at or above it. Each signal SHALL include repository identity, subject kind, subject, score, delta, `window`, threshold, evidence references as ordered `server_trace_events` row IDs, and `created_at`. The configured threshold SHALL come from server configuration and default to a deterministic value.

#### Scenario: Crossing the configured threshold persists one signal

- **GIVEN** a repository subject has cumulative score `9`
- **AND** the configured threshold is `10`
- **WHEN** an accepted event raises that subject score to `10` or higher
- **THEN** one `HotspotSignal` row is inserted
- **AND** the signal records the subject identity, new score, score delta, `window = "cumulative"`, threshold, and evidence row IDs

#### Scenario: Additional accepted events above the threshold do not create extra crossing signals

- **GIVEN** a subject already has cumulative score at or above the configured threshold
- **WHEN** another accepted event increases the same subject's score further
- **THEN** no new threshold-crossing signal is inserted unless a lower-to-higher crossing occurs again under a future additive window model

#### Scenario: Duplicate replay does not create a signal

- **GIVEN** an accepted event already caused a threshold-crossing signal
- **WHEN** the same producer event is replayed idempotently
- **THEN** no additional signal row is inserted
- **AND** existing signal history remains unchanged

### Requirement: Server-store migration is additive and cumulative-only for this foundation

This foundation SHALL add accumulator and signal tables to the server-owned database without modifying the local `.scryrs/scryrs.db` schema or local hotspot-report contract. The only live window implemented by this task SHALL be `cumulative`, but accumulator and signal records SHALL carry a `window` field/value so later recent-window work can extend the model additively. Existing `server_trace_events` rows present before the schema upgrade SHALL NOT be backfilled into accumulator state by this task.

#### Scenario: Existing server store upgrades without historical backfill

- **GIVEN** a `server.db` created before hotspot accumulator support already contains `server_trace_events` rows
- **WHEN** the upgraded server opens that database
- **THEN** the additive schema migration succeeds
- **AND** no historical accumulator rows are synthesized from the pre-upgrade events
- **AND** only newly accepted events contribute to the new accumulators

#### Scenario: Local hotspot storage remains unchanged

- **WHEN** this foundation is implemented
- **THEN** the local `.scryrs/scryrs.db` schema is unchanged
- **AND** the existing local `HotspotsReport` contract remains unchanged

### Requirement: Verification proves cumulative batch-live alignment and ingest safety

The implementation SHALL add automated tests covering accepted subject-bearing updates, lifecycle exclusion, rejected-item no-op behavior, duplicate replay, failure bonus, threshold crossing, no duplicate signal on replay, and cumulative alignment against existing `score_hotspots` / `scryrs hotspots` semantics across the supported subject-bearing event families.

#### Scenario: Full subject-bearing fixture aligns with batch output

- **GIVEN** the multi-event-family hotspot verification fixture used for batch scoring coverage
- **WHEN** the same events are ingested through the live server foundation
- **THEN** cumulative live scores match the batch scores
- **AND** cumulative live ranks match the batch ranks

#### Scenario: Mixed batches leave rejected siblings out of live state

- **GIVEN** a batch containing accepted subject-bearing events and rejected items
- **WHEN** the batch completes
- **THEN** only the accepted subject-bearing events affect accumulators
- **AND** rejected siblings do not appear in accumulator state or signal history
