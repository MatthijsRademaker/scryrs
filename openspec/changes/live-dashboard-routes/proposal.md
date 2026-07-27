## Why

Route explain is one of scryrs' core routing surfaces, but it is local-only because route manifests currently exist only as workspace files. Swarm operators see no Routes navigation and cannot inspect the same route evidence that local dashboard users can query.

## What Changes

- Define a repository-scoped live route-manifest artifact contract.
- Add an explicit client-to-server route-manifest publication path.
- Store the latest validated route manifest per repository on the live server.
- Add a live server route-explain endpoint using the stored manifest and existing deterministic `scryrs_runtime::explain_hints` logic.
- Add a dashboard live proxy for route explain.
- Enable Routes navigation and search in live mode.
- Preserve local route-manifest behavior and schema validation.
- Fail clearly when a repository has not published a manifest or publishes an incompatible schema.

## Capabilities

### New Capabilities

- `live-dashboard-routes`: Publication and live querying of repository route manifests.

### Modified Capabilities

- `dashboard-route-explain`: Route explain supports a validated live source in addition to local `.scryrs/routes.json`.
- `live-dashboard-mode`: Routes become available in live mode when a repository has a published manifest.

## Impact

- `crates/scryrs-server`: route-manifest storage, validation, publication endpoint, and explain endpoint.
- `crates/scryrs-cli` or runtime publication workflow: explicit route-manifest upload after local generation.
- `crates/scryrs-dashboard/src/route_explain.rs` and proxy handlers: local/live source selection.
- `crates/scryrs-dashboard/frontend`: live route navigation and API handling.
- Server storage schema and repository artifact lifecycle gain a versioned route-manifest record.
- No server-side graph reconstruction is included; server consumes published manifests.
