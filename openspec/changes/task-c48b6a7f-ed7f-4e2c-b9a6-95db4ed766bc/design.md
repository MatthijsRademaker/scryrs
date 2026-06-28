## Context

scryrs already generates deterministic `ProposalDocument` inbox artifacts under `.scryrs/proposals/`, and the review decision contract already exists in `scryrs-types` as `ProposalReviewDecision` plus `ReviewOutcome`. What is missing is the CLI workflow that lets a reviewer record durable accept/reject outcomes without hand-authoring JSON files.

The implementation must add a first nested CLI namespace to a dispatch layer that is currently flat, while preserving the established CLI contract: usage and input failures exit 2, error diagnostics follow the existing three-line pattern, and review commands must leave source-of-truth docs, graph, routes, memory truth, and proposal inbox artifacts untouched.

## Goals / Non-Goals

**Goals**

- Register `scryrs proposals list`, `scryrs proposals accept`, and `scryrs proposals reject` as explicit review commands.
- Keep review commands unconditionally compiled rather than gated behind the `curator` feature.
- Validate proposal inbox artifacts before writing decisions, including filename/id and computed content-address checks.
- Require explicit `--reviewer`, `--rationale`, and `--decided-at` metadata so accept/reject writes are deterministic.
- Let `list` distinguish pending, accepted, rejected, and orphan decision states with deterministic JSON output.
- Update help, help-json, snapshots, CLI docs, proposals docs, and README references for the new command surface.
- Prove through tests that review commands only write reviewed-evidence artifacts and do not mutate source-of-truth outputs.

**Non-Goals**

- Do not mutate or delete `.scryrs/proposals/{id}.json` during accept or reject.
- Do not publish accepted content into Markdown docs, ADRs, skills, playbooks, memory truth, `.scryrs/graph.json`, or `.scryrs/routes.json`.
- Do not add dashboard review UX, LLM-assisted review, auto-acceptance, or proposal editing.
- Do not implement overwrite or transition flows for already accepted or rejected proposals.
- Do not make review commands depend on the `curator` feature.

## Decisions

### Decision 1: Use a plural nested namespace for review commands

Adopt `scryrs proposals` as a new clap subcommand group with nested `list`, `accept`, and `reject` subcommands. This keeps proposal generation (`scryrs propose`) distinct from proposal review (`scryrs proposals ...`) and matches the accepted naming decision from refinement.

### Decision 2: Implement review logic in a dedicated unconditional module

Create `crates/scryrs-cli/src/proposal_review.rs` and wire it from `dispatch.rs` without feature gating. The review commands depend only on filesystem I/O plus `scryrs-types`, so they must remain available even when `curator`-gated generation code is absent.

### Decision 3: Reuse the existing review-decision contract exactly

`accept` and `reject` consume `.scryrs/proposals/{id}.json`, deserialize `ProposalDocument`, call proposal validation, then apply additional CLI-only checks: the filename stem must match `ProposalDocument.id`, and the computed content address must match `ProposalDocument::compute_id()`.

On success:

- `accept` writes `.scryrs/accepted/{id}.json` with `outcome = accepted`, copies `targetType` from the proposal, copies `proposedContent` into `acceptedContent`, copies proposal `evidence` into `sourceEvidence`, and records caller-supplied reviewer metadata.
- `reject` writes `.scryrs/rejected/{id}.json` with `outcome = rejected`, copies proposal `evidence` into `sourceEvidence`, omits `targetType`, omits `acceptedContent`, and records caller-supplied reviewer metadata.

### Decision 4: Make state listing deterministic and explicit

`list` scans proposal inbox artifacts, overlays accepted/rejected decisions, validates any matching decision artifacts, sorts rows by proposal ID, and emits a deterministic JSON envelope.

The output contract is:

- top-level `command = "proposals list"`
- top-level `schemaVersion = "0.9.0"`
- top-level `proposals` array sorted by ascending proposal ID

Each proposal-backed row includes `id`, `title`, `targetType`, `createdAt`, and `state`. Accepted/rejected rows also include `reviewer` and `decidedAt`. Orphan decision rows use `state = "orphan_accepted"` or `state = "orphan_rejected"` and include the decision metadata needed to surface that orphaned evidence.

`--state` accepts `pending`, `accepted`, `rejected`, or `all`, and defaults to `all`. The exact-state filters select proposal-backed rows in that state; `all` also includes orphan decision rows.

### Decision 5: Keep the existing fail-loud CLI contract

All review usage or input failures exit with code 2 and follow the established three-line diagnostic pattern. The command-specific usage lines are:

- `Usage: scryrs proposals list <PATH> [--state pending|accepted|rejected|all]`
- `Usage: scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`
- `Usage: scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`

This applies to missing arguments, unknown proposal IDs, malformed JSON, invalid proposal documents, filename/id mismatch, content-address mismatch, invalid `decidedAt`, invalid decision files, and existing or conflicting decisions.

### Decision 6: Preserve the review-first artifact boundary

Review commands create only `.scryrs/accepted/{id}.json` or `.scryrs/rejected/{id}.json`. They do not rewrite `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/`, or representative memory/docs source paths, and they never mutate the original proposal inbox artifact.

### Decision 7: Reuse and share deterministic test guardrails

Protected-path verification should follow the existing `verify_proposal_writes_confined` testing pattern from `propose.rs`. Because the review module needs the same file-inventory and protected-path assertions, extract or share the relevant test helpers instead of duplicating them privately.

## Risks

- Adding the first nested `proposals` namespace expands the current flat dispatch architecture and requires explicit missing-argument and unknown-argument handling for the new subcommand group.
- A single malformed proposal or decision artifact should block `list` rather than be silently skipped, so corrupted review evidence can prevent aggregate listing until fixed.
- `scryrs propose` and `scryrs proposals ...` are intentionally distinct but visually similar; help and docs must emphasize generation versus review responsibilities.
- Accept/reject are deterministic only when caller-supplied reviewer metadata is explicit and validated; there is no wall-clock fallback.

## Conflict Resolution

### Orphan decision visibility

Refinement raised two possible behaviors for decision artifacts that no longer have a matching proposal inbox file: hide them or surface them. This specification resolves that conflict in favor of surfacing them as `orphan_accepted` and `orphan_rejected` rows in `scryrs proposals list`, based on the accepted lead-developer decision. Only truly ambiguous state — both accepted and rejected decisions for the same proposal ID — fails loudly.

### List output shape

Refinement required deterministic JSON rows and separately called out the need for a versioned schema. This specification resolves that by using a versioned JSON envelope containing a sorted `proposals` array rather than an unversioned stream or plain-text listing.

### Review-contract dependency status

Refinement questioned whether the active `proposal-review-contract` change had to be archived before implementation. This specification treats that dependency as resolved because the contract is already implemented in `crates/scryrs-types` and is directly consumable by the CLI.

## Traceability

- Task `c48b6a7f-ed7f-4e2c-b9a6-95db4ed766bc`
- Exploration dossier `2026-06-28T10:00:45.724Z`
- Accepted decisions `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, and `1-swarm-reviewer-recommendation`
- Round 1 validated outputs from `swarm-architect`, `swarm-lead-dev`, and `swarm-reviewer`
- Current artifact snapshot `initial`