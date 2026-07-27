## 1. Contract and publication security

- [x] 1.1 Define live route publication and route-explain HTTP DTOs, status codes, size limits, and schema-version behavior.
- [x] 1.2 Decide and implement authentication/authorization for route-manifest publication, including repository ownership checks.
- [x] 1.3 Define atomic latest-manifest replacement and publication metadata.

## 2. Server route-manifest storage

- [x] 2.1 Add versioned repository-scoped route-manifest storage and migration.
- [x] 2.2 Add publication validation using `RouteManifestDocument` and route schema rules.
- [x] 2.3 Add `POST` publication endpoint with idempotency, size enforcement, and no-replacement-on-error behavior.
- [x] 2.4 Add `GET /v1/repositories/:repositoryId/routes/explain` using `scryrs_runtime::explain_hints`.
- [x] 2.5 Add server tests for valid publication, malformed/schema-mismatch manifests, missing manifests, deterministic hints, and repository isolation.

## 3. Client publication path

- [x] 3.1 Choose explicit CLI publication command or flag and document its authentication/configuration.
- [x] 3.2 Implement publication from the generated `.scryrs/routes.json` without changing local route generation.
- [x] 3.3 Add CLI tests for publication success, retry, server failure, and schema mismatch.

## 4. Dashboard integration

- [x] 4.1 Add live proxy behavior for `/api/routes/explain` while preserving local artifact behavior.
- [x] 4.2 Enable Routes in live navigation and adapt unavailable/error states to missing publication.
- [x] 4.3 Add frontend/API tests covering live route search, empty results, missing publication, and upstream failure.

## 5. Verification and documentation

- [x] 5.1 Add end-to-end coverage publishing a manifest, querying live hints, and verifying deterministic output.
- [ ] 5.2 Publish a manifest for an existing swarm repository and verify `scryrs.localhost/routes`.
- [ ] 5.3 Run Rust, frontend, and Docker-backed verification lanes.
- [ ] 5.4 Document manifest publication lifecycle, staleness, schema compatibility, and authentication.
