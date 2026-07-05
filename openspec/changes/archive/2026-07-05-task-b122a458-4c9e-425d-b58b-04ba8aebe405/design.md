## Context

`scryrs route explain` already loads `.scryrs/routes.json`, validates `ROUTE_SCHEMA_VERSION`, applies deterministic query matching through `scryrs_runtime::explain_hints`, and emits evidence-backed `RouteHintDocument` output. The backlog item for Runtime Retrieval 05 needs a second CLI surface that reuses that exact ranking behavior but turns it into a bounded, read-only context-loading plan. The command must stay model-free, avoid hidden file/doc reads, and preserve the evidence chain behind every returned target.

## Goals / Non-Goals

### Goals

- Expose `scryrs route bundle <PATH> --query <TEXT> --limit <N>` under the existing `route` namespace.
- Reuse the existing route-explain matching and ordering semantics instead of introducing parallel ranking.
- Emit a stable JSON bundle plan containing bounded ranked targets plus route IDs, reasons, evidence, rank, and relevance.
- Preserve explainability by keeping route ID and evidence links on every included target.
- Keep the command read-only and model-free: consume `.scryrs/routes.json` only and do not mutate runtime context.
- Update help, help-json, and developer docs so agents know when to call `bundle` versus `explain`.

### Non-Goals

- No semantic search, model-based ranking, fuzzy matching, or hidden heuristics beyond current `route explain` logic.
- No automatic context loading, source-file reads, docs reads, or context mutation by the bundle command itself.
- No regeneration of `.scryrs/routes.json` or fallback to `.scryrs/graph.json` when the route manifest is missing.
- No dashboard, server, or API surface changes.

## Decisions

### Decision 1: Add `route bundle` as a new route subcommand that follows the existing custom dispatch pattern

Implement a new `crates/scryrs-cli/src/route_bundle.rs` handler and extend the pre-clap `route` interception path in `crates/scryrs-cli/src/dispatch.rs` so `route bundle`, `route bundle --help`, and the existing `route explain` flow all route correctly without regressing `scryrs route <PATH>` manifest generation. Mirror the existing runtime feature-gating pattern used by `route explain`.

### Decision 2: Share route-manifest loading and schema validation between explain and bundle

Extract the `.scryrs/routes.json` resolution, JSON parsing, and `ROUTE_SCHEMA_VERSION` validation logic into a shared CLI helper module (for example `route_common.rs`) used by both `route_explain.rs` and `route_bundle.rs`. The helper must preserve the exact existing route-explain diagnostics and exit-code behavior.

### Decision 3: Reuse explain ordering exactly and truncate after ranking

Bundle selection comes from `scryrs_runtime::explain_hints` or a shared extraction of the same logic. Ordering remains the ordered explain result; the bundle then applies the requested bound with `take(limit)`. No new ranking, filtering, or deduplication policy is introduced before truncation.

### Decision 4: Define a first-class `RouteBundleDocument` contract

Add a new versioned bundle wire contract in `crates/scryrs-types/src/lib.rs` with its own `BUNDLE_SCHEMA_VERSION`. The envelope includes `schemaVersion`, `query`, `limit`, and `targets`. Each target reuses the existing `RouteHintItem` field shape: `routeId`, `target`, `loadTarget`, `label`, `rank`, `relevance`, `reason`, and `evidence`.

### Decision 5: Include non-loadable targets and count them toward the limit

Bundle output preserves the same explain-derived entries instead of filtering to only loadable files or docs. Targets with `loadTarget.kind = non_loadable` remain explicit in the output and count toward the requested bound.

### Decision 6: Require a positive `--limit`

`--limit` is mandatory and must be a positive integer. Missing, non-numeric, zero, or negative values fail fast with exit code 2 and explicit diagnostics consistent with `route explain`.

### Decision 7: Document bundle as the bounded planning surface and explain as the diagnostic surface

Human help, help-json, and the developer docs must distinguish the two commands clearly: `bundle` is the bounded context-loading plan, while `explain` remains the fuller diagnostic explanation of ranked route hints. Because `--help-json` is a public CLI surface, update `SURFACE_VERSION` if required by the implementation.

## Risks

- Extending the pre-clap intercept incorrectly could break existing `route explain` dispatch or cause `route bundle` to fall through to clap as an unknown or misparsed command.
- Extracting shared manifest-loading code can accidentally change the current three-line error format or exit-code behavior if the helper is not kept byte-for-byte compatible with `route explain` diagnostics.
- Adding a new help-json surface without updating the pinned version and snapshots will break CLI surface tests.
- Bundle must stay aligned with the existing runtime feature-gating assumptions used by `route explain` so non-default builds keep linking correctly.

## Conflict Resolution

### `--limit 0`

Refinement conflicted on whether `--limit 0` should be valid. This specification resolves the conflict in favor of exit code 2 for zero because the dossier assumptions and accepted acceptance criteria already require missing, invalid, and non-positive limits to fail fast, and the reviewer explicitly flagged the zero-valid alternative as contradictory to that contract.

### Shared helper vs. duplicated manifest loading

Refinement conflicted on whether bundle should duplicate `route_explain.rs` manifest loading or extract a shared helper. This specification resolves the conflict in favor of a shared loader because the accepted architect decision called for a shared `route_common` helper, the reviewer warned about drift if the loading contract forks, and both commands must preserve the same `.scryrs/routes.json` validation and diagnostics.

## Traceability

- Task: `b122a458-4c9e-425d-b58b-04ba8aebe405`
- Dossier: `2026-07-05T18:21:10.755Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base: `task-b122a458-4c9e-425d-b58b-04ba8aebe405` snapshot `initial`
