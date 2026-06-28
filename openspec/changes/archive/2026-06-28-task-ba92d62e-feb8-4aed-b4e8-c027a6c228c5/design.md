## Context

The scryrs codebase already provides:
- `scryrs route <PATH>` — generates `.scryrs/routes.json` (a `RouteManifestDocument`) from `.scryrs/graph.json`.
- `hints_from_manifest(manifest) -> RouteHintDocument` in `crates/scryrs-runtime` — pure, deterministic projection from route entries to hint items.
- `RouteHintDocument` / `RouteHintItem` wire contracts in `crates/scryrs-types` — identity, target, rank, reason, evidence.
- Proven CLI patterns: `proposals` pre-clap intercept for nested subcommands, `route.rs` fail-fast artifact loading with exit-2 diagnostics.

What is missing is the `scryrs route explain` CLI surface and a query-aware filtering layer over those hints. Current help, help-json, and docs explicitly say this command is deferred.

## Goals / Non-Goals

### Goals
1. A documented CLI surface `scryrs route explain <PATH> --query <TEXT>` that agents and users can invoke.
2. Deterministic, byte-identical output for identical inputs — no model, no randomness.
3. Reuse of existing `RouteManifestDocument` input, `RouteHintDocument` output, and `hints_from_manifest` projection.
4. Fail-fast for missing, malformed, or schema-mismatched `.scryrs/routes.json` (exit 2 with explicit diagnostics).
5. Case-insensitive substring matching with documented tie-break: exact > prefix > substring, manifest entry order as final tie-break.
6. Query-match provenance visible in each `RouteHintItem.reason` field.
7. Updated help, help-json, docs, and snapshots with copy-paste example and interpretation notes.

### Non-Goals
- No inspection of `.scryrs/graph.json` or proposal/review artifacts.
- No model-based ranking, fuzzy semantic retrieval, or hidden heuristics.
- No mutation of `.scryrs/routes.json` or canonical OpenSpec specs.
- No redesign of the route-hint wire contract — the existing schema is reused as-is.
- No `--limit` flag, no `relevance` population, no grouping group-label matching in this implementation.

## Decisions

### Decision 1: CLI dispatch — pre-clap intercept + parent command restructuring

**Choice**: Use the `proposals` pre-clap intercept pattern (check `args[0] == "route" && args[1] == "explain"` before clap processes args) combined with restructuring the `route` clap subcommand as a parent command with `.subcommand_required(false)`. This preserves `scryrs route <PATH>` backward compatibility while adding `route explain` as a nested subcommand.

**Alternatives considered**:
- Make `explain` a sibling top-level command — rejected because it fragments the namespace and obscures the relationship between route generation and route explanation.
- Make `--explain` a flag on the existing `route` command — rejected because explain consumes a different artifact (routes.json vs graph.json) and has different arg requirements.

**Rationale**: The `proposals` pattern is proven in this codebase. The pre-clap intercept catches `route explain` before clap's flat subcommand model processes it, avoiding interference with clap's arg parsing for the existing `route <PATH>` subcommand.

### Decision 2: Query matching algorithm

**Choice**: Case-insensitive substring matching over `RouteEntry.label`, `subject`, `id`, `target`, `kind`, and `evidence_links[].subject`. Matches are tiered: exact match first, then prefix match, then substring match. Within each tier, manifest entry order (by id ascending) is the final tie-break. Results are filtered to only entries that match (no partial/no-match entries in output).

**Searched fields**: `label`, `subject`, `id`, `target`, `kind`, `evidence_links[].subject`. NOT `grouping.group_label`.

**Rationale**: This is purely deterministic, model-free, and uses fields already present in `RouteEntry` and its evidence links. Case-insensitive matching is more practical for agents while remaining deterministic and locale-independent. The tiered ordering gives intuitive results (exact label matches appear first) while preserving deterministic byte-identical output.

### Decision 3: Reason template extension

**Choice**: Append query-match provenance to the existing reason template: `"Route '{label}' ({id}): {N} evidence link(s), subject kind {subjectKind}; query match on {fields}"` where `{fields}` is a comma-separated list of matched field names (e.g., `"label, evidence.subject"`).

**Rationale**: The existing template is preserved for `hints_from_manifest`. The explain producer adds query-match context showing exactly which fields matched, fulfilling the acceptance criterion that output "cites route-manifest and evidence sources." No schema change is needed — the `reason` field is already a free-form string.

### Decision 4: No-match contract

**Choice**: Zero query matches produces a valid `RouteHintDocument` with an empty `hints` array, written to stdout, with exit code 0. No error or warning on stderr.

**Rationale**: An empty result is a valid deterministic output (not a failure), consistent with how `hints_from_manifest` handles an empty manifest and how `scryrs route` handles empty graphs. Exit 0 for valid output; exit 2 only for input artifact problems.

### Decision 5: `explain_hints` as a new function in scryrs-runtime

**Choice**: Add a new public function `explain_hints(manifest: &RouteManifestDocument, query: &str) -> RouteHintDocument` in `crates/scryrs-runtime/src/lib.rs`. This function calls `hints_from_manifest` internally, then filters and re-orders results by query match.

**Rationale**: `hints_from_manifest` remains a pure ordinal projection unchanged. The explain producer composes with it and adds only the query-filter layer. This keeps the existing API stable for other consumers while making the explain logic testable in isolation.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Backward-compat break: `scryrs route <PATH>` stops working after CLI restructuring | High | `.subcommand_required(false)` on parent + pre-clap intercept only for `route explain`. Existing dispatch tests for `route <PATH>` must pass unchanged. |
| Non-deterministic filtering: HashMap or HashSet iteration in query matcher | Medium | Use `Vec` + `BTreeMap` or explicit sort. Match-tier comparison must use stable sort with manifest-order tie-break. |
| Snapshot update burden: 2 insta snapshots reference `route explain` as deferred; missing snapshot update breaks CI | Low | Both snapshots are touched deliberately. `cargo test` with `INSTA_UPDATE=always` after all help-text changes. |
| API surface creep: `explain_hints` might seem like a public API that downstreams depend on | Low | Document as `#[doc(hidden)]` or mark as internal. The CLI is the primary consumer. |

## Traceability

- Task ID: `ba92d62e-feb8-4aed-b4e8-c027a6c228c5`
- Artifact snapshot: `proposal-synthesis-input` at `2026-06-28T20-43-28-810Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Validated round outputs: architects (round 1), lead-dev (round 1), reviewer (round 1)
- Non-blocking resolutions: Reviewer blocker-1 (schema/reason template), Reviewer blocker-2 (policy/match algorithm)