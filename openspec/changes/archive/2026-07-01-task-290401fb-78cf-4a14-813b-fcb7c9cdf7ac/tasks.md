## 1. Route and hint contracts

- [x] Add a typed optional `loadTarget` contract to `RouteEntry` and `RouteHintItem` while keeping `target` as the stable graph-node identity string and leaving schema versions unchanged.
- [x] Define the three supported load-target cases: `file`, `doc_page`, and `non_loadable`.

## 2. Route generation

- [x] Update `crates/scryrs-cli/src/route.rs` so `file` routes emit validated repository-relative file references, `doc_page` routes emit canonical `project-docs/<slug>` references, and `search` / `symbol` / `domain_term` / `doc_group` routes emit explicit `non_loadable` targets.
- [x] Fail `scryrs route <PATH>` with exit code 2 when a `file` or `doc_page` route cannot produce the promised load target.
- [x] Keep `RouteEntry.id`, `RouteEntry.target`, grouping behavior, evidence ordering, route ordering, and `crates/scryrs-cli/src/graph.rs` behavior unchanged.

## 3. Hint and explain projection

- [x] Copy `loadTarget` into `hints_from_manifest` and explain output.
- [x] Append `, load target <kind>` to the base hint / explain reason template without changing explain match fields, explain ranking rules, or read-only manifest consumption.

## 4. Documentation and verification

- [x] Update help text, help-json, `.devagent/docs/docs/route-manifests.md`, `.devagent/docs/docs/cli-v0-contract.md`, and the OpenSpec deltas for `route-manifest`, `route-hint`, and `route-explain`.
- [x] Add tests covering file, malformed file, doc_page, search, symbol, and domain_term routes across route generation, hint projection, and explain output, including explicit non-loadable behavior.
