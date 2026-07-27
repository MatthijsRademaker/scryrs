## Context

Local route explain reads `.scryrs/routes.json`, produced from `.scryrs/graph.json` by `scryrs route`. The live server currently stores trace events and hotspot accumulators only; it cannot reconstruct route entries because route generation depends on the client workspace graph, repository-relative files, and documentation references.

## Goals / Non-Goals

**Goals:**

- Publish a validated, versioned `RouteManifestDocument` per repository.
- Store the latest manifest durably and expose deterministic live explain queries.
- Reuse `scryrs_runtime::explain_hints` rather than duplicate ranking logic.
- Make Routes navigation and search available in live mode.
- Keep local route generation and local dashboard behavior unchanged.

**Non-Goals:**

- Server-side graph reconstruction from raw trace events.
- Server access to repository source files or project docs.
- Automatic publication on every trace event.
- Editing route manifests through the dashboard.

## Decisions

1. **Publish manifests, not graphs.** Clients upload the already-derived `RouteManifestDocument`. Rebuilding graphs on the server would require source/docs access and duplicate client graph semantics. Uploading `graph.json` was rejected because route consumers need the stable route contract, not an internal graph artifact.

2. **Use repository-scoped latest-artifact storage.** Store one latest validated manifest per repository, with schema version, content, publisher metadata, and update timestamp. Historical versions are out of scope initially; replacing a manifest is an atomic write.

3. **Separate publication from querying.** Add a write endpoint for trusted publication and a read-only explain endpoint. Dashboard only calls the read endpoint. The publication path can later be called by `scryrs route` or an explicit upload command without coupling route generation to every dashboard request.

4. **Validate at ingestion and query boundaries.** Publication rejects malformed JSON, schema mismatch, invalid route references, and oversized payloads. Query returns a clear not-published response when no manifest exists. The dashboard does not silently fall back to local files in live mode.

5. **Reuse deterministic runtime matching.** The server invokes `scryrs_runtime::explain_hints` against the stored manifest. A second server-specific matcher was rejected because it would create ranking drift between local and live dashboards.

6. **Treat publication authorization as required.** Repository ID is routing identity, not authorization. The deployment must authenticate publication requests before enabling the write endpoint outside trusted networks.

## Risks / Trade-offs

- **[Stale manifest]** Live routes may lag workspace changes → expose publication timestamp/version in metadata and document explicit republish behavior.
- **[Untrusted upload]** A caller could overwrite another repository's routes → require server authentication and bind publisher identity to repository authorization.
- **[Large manifests]** Route graphs may exceed practical request/storage limits → enforce a documented maximum and return 413 before parsing/storage.
- **[Schema mismatch]** New clients may publish unsupported versions → reject with explicit expected-version diagnostics; retain local behavior.
- **[Runtime drift]** Server and client runtime versions may rank differently → keep shared crate/version aligned in server builds and add golden explain fixtures.

## Migration Plan

1. Add additive manifest storage and publication/read APIs.
2. Add client publication command/integration; existing `scryrs route` remains local-only until explicitly followed by publication.
3. Add dashboard live proxy and Routes navigation behind successful read-path tests.
4. Publish manifests for existing swarm repositories.
5. Roll back by hiding live Routes navigation and retaining stored manifests; no local artifact migration is needed.

## Open Questions

- Should publication be an explicit `scryrs route publish` command or part of `scryrs route` behind a flag?
- What authentication mechanism does swarm deployment already provide for write APIs?
- Should server retain manifest history for comparing route changes, or only the latest version?
