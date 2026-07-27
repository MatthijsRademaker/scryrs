## Context

Proposal documents and review decisions currently live under `.scryrs/proposals`, `.scryrs/accepted`, and `.scryrs/rejected`. The local dashboard reuses curator inventory and review-write semantics. Live mode has no filesystem artifacts and therefore cannot expose proposal state or accept/reject decisions.

## Goals / Non-Goals

**Goals:**

- Publish validated proposal documents to a repository-scoped server store.
- Read proposal inventory and detail from live mode.
- Persist explicit accept/reject decisions with idempotent conflict semantics.
- Preserve proposal schema, evidence, and review-first behavior.
- Make remote review auditable and repository-scoped.

**Non-Goals:**

- Publishing accepted knowledge to Markdown/Rspress outputs.
- Replacing local curator artifacts or local CLI review behavior.
- Anonymous remote review writes.
- Automatic proposal generation on the server.

## Decisions

1. **Persist structured JSON records, not opaque files only.** Store validated proposal and review fields in server-owned tables (or a versioned JSON record store) with repository ID and proposal ID keys. Structured storage enables deterministic inventory, conflict detection, and indexed lookup. Opaque blobs alone were rejected because review state would require reparsing every request.

2. **Reuse curator validation and serialization.** Extract or expose shared validation/build functions from `scryrs-curator`; the server must reject invalid proposal/review payloads before persistence. A second validator was rejected because local and live workflows must agree on schema and target-type rules.

3. **Use explicit publication and review APIs.** Publication is a client-to-server write; dashboard list/detail are reads; accept/reject are explicit writes. Dashboard does not infer proposals from trace events and does not publish accepted content.

4. **Preserve local review conflict semantics.** First terminal decision wins. Byte-identical same-outcome retries return success; opposite outcomes or changed same-outcome payloads return conflict. No silent overwrite.

5. **Require authenticated reviewer identity.** `repository_id` is not authorization. Proposal publication and review writes require deployment authentication, while read access follows the dashboard's live access policy. This is mandatory before exposing remote review beyond a trusted network.

6. **Keep frontend API shape stable.** Dashboard proxies continue to expose `/api/proposals` and `/api/proposals/:id` plus accept/reject routes. The frontend switches source through mode-aware stores rather than maintaining a second live-only component tree.

7. **Use repository-bound opaque bearer credentials for writes.** `SCRYRS_PROPOSAL_WRITE_CREDENTIALS` configures server-side `{repositoryId, actorId, token}` records. Tokens are SHA-256 digested in memory, compared in constant time, and accepted only for their configured repository. Publication and review require `Authorization: Bearer <token>`; review additionally requires request `reviewer` to equal authenticated `actorId`. Dashboard and CLI client tokens resolve only from `SCRYRS_PROPOSAL_WRITE_TOKEN` or `.scryrs/.env`, never command-line flags or committed config.

8. **Use proposal/review schema 1.0.0 with bounded canonical JSON.** Proposal publication accepts at most 1 MiB; review requests accept at most 64 KiB. Malformed JSON or semantic validation returns 400, incompatible schema/target content returns 422, oversized payloads return 413, missing/invalid auth returns 401, repository mismatch returns 403, unknown proposals return 404, revision or terminal-decision mismatch returns 409, disabled writes return 503, and persistence failures return 500/502 through server/dashboard boundaries.

9. **Treat canonical document hashes as immutable revisions.** Server serializes validated documents to compact canonical struct JSON and stores SHA-256 revisions. Re-publishing same proposal ID and hash is idempotent; same ID with another hash conflicts. First terminal review wins; identical canonical decision retries are idempotent and every other retry conflicts.

10. **Retain immutable audit records.** Proposal audit fields are repository ID, proposal ID, revision SHA-256, publisher actor ID, schema version, and first publication timestamp. Review audit fields add authenticated actor ID, explicit reviewer, outcome, decision SHA-256, decision timestamp, and server recording timestamp. Idempotent retries do not rewrite audit fields. No proposal/review delete API exists; rollback disables routes/navigation while records remain retained.

## Risks / Trade-offs

- **[Remote write security]** Unauthorized users could accept/reject proposals → require authentication, authorization, audit identity, and deployment-level transport security before enabling writes.
- **[Schema evolution]** Stored proposals can outlive code versions → persist schema version and reject incompatible writes without corrupting existing records.
- **[Duplicate publication]** Retries may create duplicate rows → unique `(repository_id, proposal_id)` key and idempotent same-content publication.
- **[Large proposal content]** Markdown or structured content may be oversized → enforce request and per-proposal size limits with explicit 413 errors.
- **[Local/live divergence]** A workspace may review locally after publishing → include revision/content hash and expose conflict diagnostics rather than silently merging.

## Migration Plan

1. Add additive proposal/review storage and validation APIs.
2. Add explicit publication integration for curator-generated proposals.
3. Add live read proxy and read-only Proposals navigation.
4. Add authenticated review writes after deployment auth is verified.
5. Keep local filesystem dashboard behavior unchanged throughout rollout.
6. Roll back by disabling live proposal navigation and write routes; retain stored records for later retry.

## Resolved Questions

- Write auth uses repository-bound opaque bearer credentials with auditable actor IDs.
- Live reads remain available without write credentials; review controls depend on write capability.
- Publication uses canonical proposal SHA-256 revisions and refuses replacement.
- `scryrs-curator::proposals::{inventory, review_write}` owns shared validation and decision construction.
