## Why

`scryrs route explain` already produces deterministic, evidence-backed route hints from `.scryrs/routes.json`, but it is an unbounded diagnostic surface. Runtime Retrieval 05 needs a separate bounded context-loading plan that future agents can call when they need a small, explainable set of files or docs to inspect without hidden reads, model ranking, or automatic context mutation.

## What Changes

### 1. Add a bounded `scryrs route bundle` CLI surface

Introduce `scryrs route bundle <PATH> --query <TEXT> --limit <N>` under the existing `route` namespace. The new subcommand follows the same pre-clap interception pattern as `route explain`, keeps `scryrs route <PATH>` manifest generation unchanged, and mirrors the existing runtime feature-gating behavior.

### 2. Reuse route-explain matching and ordering exactly

`route bundle` will load only `.scryrs/routes.json`, validate `ROUTE_SCHEMA_VERSION`, call the existing `scryrs_runtime::explain_hints` logic (or a shared extraction of the same logic), and truncate the ordered results with the requested positive limit. No second ranking algorithm, fuzzy matching, semantic search, or hidden source/doc reads are added.

### 3. Publish a first-class bundle wire contract

Add a versioned `RouteBundleDocument` in `scryrs-types` with `schemaVersion`, `query`, `limit`, and a bounded `targets` array. Each target preserves the explain-derived `routeId`, `target`, `loadTarget`, `label`, `rank`, `relevance`, `reason`, and `evidence` fields. Non-loadable targets remain explicit and count toward the limit.

### 4. Preserve existing error and read-only behavior

Bundle will fail fast with exit code 2 for missing PATH, missing query, missing or invalid limit values, missing `.scryrs/routes.json`, malformed JSON, and route schema mismatches, matching the explicit route-explain error contract. The command remains read-only and never mutates runtime context or `.scryrs/` artifacts.

### 5. Update help, docs, and verification

Update `--help`, `route bundle --help`, `--help-json`, `route-manifests.md`, and `cli-v0-contract.md` so agents know when to call `bundle` (bounded context-loading plan) versus `explain` (unbounded diagnostic ranking). Add tests and snapshots covering discoverability, stable truncation, zero matches, invalid limits, artifact-only reads, and unchanged explain ordering before truncation.

## Impact

- Adds a new read-only CLI command for bounded context planning without changing `route explain` ranking semantics.
- Introduces a versioned JSON contract for bundle consumers.
- Keeps explainability intact by preserving route IDs, reasons, load targets, and evidence in every returned target.
- Clarifies agent guidance so `bundle` is the small planning surface and `explain` remains the fuller diagnostic surface.
