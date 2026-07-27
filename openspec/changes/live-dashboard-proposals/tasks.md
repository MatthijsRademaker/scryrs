## 1. Security and contract decisions

- [x] 1.1 Select authentication and authorization mechanism for proposal publication and review writes.
- [x] 1.2 Define repository-scoped proposal/review DTOs, status codes, size limits, schema versions, and revision/hash semantics.
- [x] 1.3 Define audit fields required for remote review and retention behavior.

## 2. Shared validation and server storage

- [x] 2.1 Extract or expose curator proposal/review validation and serialization for server use.
- [x] 2.2 Add versioned repository-scoped proposal and review storage with unique proposal IDs.
- [x] 2.3 Add authenticated proposal publication with identical-content idempotency and revision conflict handling.
- [x] 2.4 Add repository-scoped list and detail endpoints with deterministic ordering.
- [x] 2.5 Add authenticated accept/reject endpoints preserving local validation, first-terminal-decision, and conflict semantics.
- [x] 2.6 Add server tests for malformed payloads, duplicate publication, cross-repository isolation, review conflicts, and idempotent retries.

## 3. Curator publication integration

- [x] 3.1 Choose explicit publication command or curator workflow integration.
- [x] 3.2 Publish proposal documents with evidence and schema metadata.
- [x] 3.3 Add retry and server-error handling without deleting or rewriting local artifacts.

## 4. Dashboard integration

- [x] 4.1 Add live proxy handlers for proposal list, detail, accept, and reject.
- [x] 4.2 Extend frontend API clients/stores for live proposal reads and review writes.
- [x] 4.3 Enable Proposals navigation only when live read capability is available.
- [x] 4.4 Render authorization, validation, conflict, and upstream failure states explicitly.
- [x] 4.5 Add frontend tests for live inventory, detail, review success, retry, and conflict behavior.

## 5. Verification and documentation

- [x] 5.1 Add end-to-end coverage publishing a proposal, viewing it live, and recording an authenticated review.
- [x] 5.2 Verify local filesystem proposal behavior remains unchanged.
- [x] 5.3 Run Rust, frontend, security, and Docker-backed verification lanes.
- [x] 5.4 Document remote proposal publication, authorization, auditability, and rollback behavior.
