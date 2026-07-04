## Why

Route manifests and route hints currently preserve graph-node identity but do not expose a directly loadable address. `target` still carries the node ID string for every subject kind, which means runtime retrieval sees values such as `file:src/auth.rs` or `doc_page:graph` instead of a repository-relative file path or a docs reference it can read.

This change lets file and docs routes advertise concrete load targets while keeping existing node IDs stable, leaving non-loadable kinds explicit, and preserving the current read-only/runtime-retrieval boundary.

## What Changes

- Keep `RouteEntry.id`, `RouteEntry.target`, `RouteHintItem.routeId`, and `RouteHintItem.target` as the existing graph-node identity strings.
- Add an optional typed `loadTarget` contract to `RouteEntry` and `RouteHintItem` with three cases:
  - `file` with repository-relative `reference`
  - `doc_page` with canonical docs `reference` `project-docs/<slug>`
  - `non_loadable` with no fake reference
- Derive file load targets from the `file:` subject using syntax-only validation: non-empty, not absolute, and no parent traversal. Do not check on-disk existence.
- Derive docs load targets from the first `DocReference` evidence link, normalizing either slug-form or prefixed inputs to `project-docs/<slug>`.
- Fail route generation with exit code 2 when a `file` or `doc_page` route cannot produce the promised load target.
- Propagate `loadTarget` through `hints_from_manifest` and `scryrs route explain`, and append `, load target <kind>` to the existing reason template before any explain query-match suffix.
- Keep `search`, `symbol`, `domain_term`, and `doc_group` routes explicit via `loadTarget.kind = "non_loadable"`.
- Update route-manifest, route-hint, route-explain documentation and tests to cover file, doc_page, search, symbol, and domain_term behavior.

## Impact

- Touches `crates/scryrs-types`, `crates/scryrs-cli/src/route.rs`, and `crates/scryrs-runtime`.
- Leaves `crates/scryrs-cli/src/graph.rs` and `crates/scryrs-graph` unchanged.
- Keeps route and hint schema versions unchanged because `loadTarget` is additive and optional.
- Requires documentation and test updates, but does not add runtime retrieval policy, graph-search changes, fake paths, or ID renames.
