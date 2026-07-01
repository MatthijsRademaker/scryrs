## Context

Current route manifests keep graph identity stable by serializing `RouteEntry.target` as the node ID string. That is correct for identity and explain matching, but it is not enough for runtime retrieval, which needs a loadable file path or docs reference. Refinement established that this task must add structured load-target metadata without changing graph build, without inventing fake targets for non-loadable subjects, and without coupling route generation to filesystem existence checks.

## Goals / Non-Goals

**Goals**

- Expose repository-relative file load references for `file` routes.
- Expose canonical docs references for `doc_page` routes.
- Preserve existing `id` / `target` / `routeId` identity strings and current explain match fields.
- Make non-loadable kinds explicit instead of manufacturing paths.
- Surface load target type in hint and explain output.
- Fail loudly only when a load-promising kind cannot produce a valid load target.
- Cover file, doc_page, search, symbol, and domain_term behavior in tests and docs.

**Non-Goals**

- No runtime retrieval policy, automatic loading, ranking, or graph search changes.
- No graph-node ID renames, subject collapsing, or fake cross-domain links.
- No changes to `crates/scryrs-cli/src/graph.rs` or `crates/scryrs-graph`.
- No file existence checks for stale hotspot routes.
- No fake repository paths or docs links for `search`, `symbol`, `domain_term`, or `doc_group` routes.

## Decisions

### Decision 1: Add a dedicated optional `loadTarget` field to route and hint contracts

`RouteEntry` and `RouteHintItem` gain an optional top-level `loadTarget` field. `target` remains the stable node-ID string. This keeps existing identity and query matching intact while making loadability a first-class, typed contract instead of an untyped metadata convention.

### Decision 2: Use a minimal typed `RouteLoadTarget` shape

`loadTarget` serializes as a structured object with `kind` and optional `reference`:

- `{"kind":"file","reference":"src/auth.rs"}`
- `{"kind":"doc_page","reference":"project-docs/graph"}`
- `{"kind":"non_loadable"}`

This is the full worker-facing contract for this task. No extra fake paths or secondary aliases are introduced.

### Decision 3: File load targets are syntax-validated only

For `file` routes, route generation derives the `reference` from the subject after `file:` and rejects empty subjects, absolute paths, and parent-traversing paths. The implementation must not require the file to exist on disk, because hotspot evidence can outlive renamed or deleted files.

### Decision 4: Docs load targets derive from `DocReference` evidence

For `doc_page` routes, route generation uses the first `EvidenceLink` with `sourceKind = doc_reference` and a usable `docRef`. The canonical reference is `project-docs/<slug>`. Inputs already expressed as `project-docs/<slug>` normalize to the same canonical form. A `doc_page` route that cannot produce a usable docs reference is invalid and must fail route generation loudly.

### Decision 5: Non-loadable kinds stay explicit

`search`, `symbol`, `domain_term`, and `doc_group` routes emit `loadTarget.kind = "non_loadable"` with no `reference`. This explicitly tells consumers that the route is meaningful for identity and evidence, but not directly loadable.

### Decision 6: Hint and explain outputs copy `loadTarget` and mention its kind

`hints_from_manifest` copies `loadTarget` into every `RouteHintItem`. Both plain projection and explain output append `, load target <kind>` to the existing base reason text. `scryrs route explain` keeps its current matching and ordering rules; `loadTarget` is additional consumer context, not a new match field or ranking input.

### Decision 7: Treat the change as additive, not a schema-version bump

`ROUTE_SCHEMA_VERSION` and `HINT_SCHEMA_VERSION` remain unchanged because `loadTarget` is optional and additive. New manifests expose the field; older manifests remain readable but will not provide load-target context until they are regenerated.

## Risks

- Consumers may mistake `project-docs/<slug>` for a repository filesystem path. Docs must state that it is an agent-facing docs reference.
- Old `.scryrs/routes.json` artifacts remain schema-valid without `loadTarget`; consumers that need load-target context must regenerate them.
- If reason-template updates are only partially documented, downstream consumers may keep stale string expectations.
- File load targets can still become unreadable at runtime if the underlying file was deleted later; that is an intentional consequence of syntax-only validation.

## Conflict Resolution

Refinement included a direct conflict between a metadata-map approach and a dedicated top-level field. This spec chooses the dedicated optional `loadTarget` field because the accepted architect recommendation and reviewer clarification both treated loadability as a core route contract, not an extension-map concern. The metadata-map alternative was rejected because it would hide a first-class wire contract behind untyped keys and make hint/manifest consumers more brittle.

## Traceability

- Task `290401fb-78cf-4a14-813b-fcb7c9cdf7ac` defines the feature, scenarios, technical notes, and acceptance criteria.
- Dossier `2026-07-01T19:28:22.391Z` defines affected areas, assumptions, open questions, and the proposal sketch.
- Accepted decision `1-swarm-architect-recommendation` defines the typed `loadTarget` approach, syntax-only file validation, canonical docs reference, non-loadable handling, and reason-template update.
- Accepted decision `1-swarm-reviewer-recommendation` requires explicit resolution of field shape, docs-reference format, file validation rules, and `RouteHintItem` propagation.
- Round output `round:1:agent:swarm-lead-dev` confirmed the no-file-existence rule and the `project-docs/<docRef>` canonical docs reference, even though its metadata-placement recommendation was not selected.
