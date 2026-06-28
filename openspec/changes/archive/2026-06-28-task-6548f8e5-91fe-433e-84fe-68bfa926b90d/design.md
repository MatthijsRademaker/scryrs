## Context

scryrs already delivers deterministic proposal generation into `.scryrs/proposals/` and already defines the `ProposalReviewDecision` contract in `crates/scryrs-types`, but it still lacks the CLI workflow that lets reviewers move proposal inbox artifacts into durable accepted or rejected review states. The project documentation and prior change state already establish the intended boundary: proposals remain review-only inbox artifacts, accepted and rejected decisions live in separate `.scryrs/accepted/` and `.scryrs/rejected/` directories, and downstream graph, route, docs, and memory publishing remain deferred.

The implementation therefore needs to add a review command surface on top of the existing proposal and review-decision contracts, while preserving deterministic writes, fail-loud behavior, and byte-for-byte non-mutation guarantees outside the reviewed-artifact paths.

## Goals / Non-Goals

### Goals

- Register the final grouped CLI surface as `scryrs proposals list|accept|reject`.
- Require explicit `--reviewer`, `--rationale`, and `--decided-at` metadata for both terminal review commands.
- List proposal IDs with deterministic pending, accepted, and rejected states, plus stable proposal metadata for machine consumers.
- Write accepted and rejected `ProposalReviewDecision` artifacts into their separate reviewed-artifact directories without mutating the proposal inbox.
- Define explicit idempotency and conflicting-terminal-state behavior before implementation.
- Extend human help, `--help-json`, docs, and snapshots to match the final command surface.
- Reuse the existing inventory-based no-mutation guard so review commands prove they do not touch docs, graph, routes, or proposal inbox files.

### Non-Goals

- No proposal generation changes and no renaming of the existing `scryrs propose` command.
- No lifecycle fields added to `ProposalDocument` and no mutation of files under `.scryrs/proposals/` during review.
- No graph build, route generation, docs publishing, memory mutation, or dashboard review UX.
- No LLM-assisted policy decisions or automatic acceptance/rejection.
- No deletion of proposals when they are rejected.

## Decisions

### Decision 1: Use a grouped `proposals` review surface and keep `propose` separate

The final command naming is:

- `scryrs proposals list <PATH> [--state pending|accepted|rejected|all]`
- `scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`
- `scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`

This intentionally introduces a grouped plural review surface while keeping the existing singular `propose` generation command unchanged. The change must document that distinction rather than trying to flatten review commands into a second naming scheme.

### Decision 2: Review metadata is required and never clock-derived

`--reviewer`, `--rationale`, and `--decided-at` are required for both `accept` and `reject`. `decidedAt` is not derived from wall-clock time, and no deterministic defaults are introduced for reviewer or rationale. This preserves deterministic writes and makes review provenance explicit.

### Decision 3: Review commands operate only on valid proposal documents

Before any review decision is written, the source proposal must:

- exist at `.scryrs/proposals/{proposalId}.json`
- deserialize as `ProposalDocument`
- pass `ProposalDocument::validate()`

Unknown IDs, malformed JSON, schema-version mismatches, or semantic proposal-validation failures all exit with code `2` and write no review-decision artifact.

### Decision 4: `proposals list` emits deterministic JSON with stable metadata

`proposals list` returns a machine-readable JSON array sorted by `proposalId` ascending. Each row includes:

- `proposalId`
- `title`
- `targetType`
- `createdAt`
- `state` (`pending`, `accepted`, or `rejected`)

The optional `--state` filter accepts only `pending`, `accepted`, `rejected`, or `all`. Invalid filters exit `2`.

### Decision 5: Conflict handling is explicit and deterministic

The review workflow uses the following policy matrix:

- repeated `accept` with byte-identical resulting artifact: success, deterministic no-op
- repeated `reject` with byte-identical resulting artifact: success, deterministic no-op
- same-outcome rerun that would produce different artifact bytes: conflict, exit `2`, no overwrite
- `accept` when a rejected artifact already exists: conflict, exit `2`, no write
- `reject` when an accepted artifact already exists: conflict, exit `2`, no write
- both accepted and rejected artifacts present for one proposal ID: conflicting terminal state; `list` fails loudly with exit `2`

This avoids silent overwrites and avoids guessing precedence when reviewed states disagree.

### Decision 6: Accepted and rejected artifacts reuse the existing review-decision contract exactly

`accept` writes `.scryrs/accepted/{proposalId}.json` as a valid `ProposalReviewDecision` with:

- `outcome = accepted`
- `targetType` copied from the proposal
- `acceptedContent` copied exactly from `proposedContent`
- `sourceEvidence` copied from the proposal evidence array

`reject` writes `.scryrs/rejected/{proposalId}.json` as a valid `ProposalReviewDecision` with:

- `outcome = rejected`
- no `targetType`
- no `acceptedContent`
- `sourceEvidence` copied from the proposal evidence array

### Decision 7: `--help-json` must represent the grouped surface directly

Because `proposals` is a real grouped root command, `--help-json` must expose it as a grouped entry with nested `list`, `accept`, and `reject` subcommands rather than pretending the surface is flat. The machine-readable surface version is bumped from `0.8.0` to `0.9.0` to reflect the structural addition.

### Decision 8: Review commands are unconditional CLI surface, not curator-only

Unlike `scryrs propose`, the review commands depend on shared proposal/review contracts rather than curator generation logic. They therefore belong on the unconditional CLI path and must not be hidden behind the `curator` feature gate.

### Decision 9: No-mutation verification is shared across proposal generation and review

The existing inventory-based protected-path test pattern used by proposal generation must be reused for review commands. The implementation should extract shared test helpers or otherwise share the inventory-comparison approach so proposal review and proposal generation enforce the same byte-for-byte boundary against `.devagent/docs/`, `.scryrs/graph.json`, `.scryrs/routes.json`, and `.scryrs/proposals/`.

## Conflict Resolution

- **Grouped vs flat command naming**: refinement raised flat `proposal-list|proposal-accept|proposal-reject` as a lower-risk implementation path, but accepted architect and lead-dev decisions both chose the grouped `scryrs proposals list|accept|reject` surface. This spec adopts the grouped surface and requires the naming inconsistency with singular `propose` to be documented rather than avoided.
- **Required `decidedAt` vs clock-derived timestamp**: refinement treated clock-derived timestamps as an open question, but accepted decisions consistently favored a required `--decided-at` argument for deterministic writes. This spec resolves the question in favor of explicit required metadata.
- **List output schema**: refinement debated whether listing should expose IDs only or richer metadata. The final spec adopts a deterministic JSON row schema containing `proposalId`, `title`, `targetType`, `createdAt`, and `state`, which matches the strongest round recommendations while staying grounded in existing `ProposalDocument` fields.
- **`--help-json` shape for grouped commands**: refinement identified the need to settle the machine surface before implementation. Because the command surface itself is grouped, this spec resolves the ambiguity by requiring a nested `proposals` entry with subcommands and a `surfaceVersion` bump to `0.9.0`.

## Risks

- Adding a grouped `proposals` root introduces the first nested CLI dispatch path in a codebase that has otherwise been flat, so dispatch and clap error routing must be updated carefully.
- A conflicting terminal state has no automatic repair path in this task; the CLI must fail loudly rather than guessing precedence.
- Extending `--help-json` to represent nested commands changes the machine-readable surface shape and must be reflected consistently in snapshots and docs.
- Duplicating protected-path test logic would drift over time; the implementation should share the existing inventory-based boundary checks.

## Traceability

- Task: `6548f8e5-91fe-433e-84fe-68bfa926b90d`
- Dossier: `2026-06-28T15:41:17.852Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round evidence: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Interpreted source boundaries: `openspec/specs/proposal-contract/spec.md`, `openspec/specs/proposal-generation/spec.md`, `openspec/changes/task-5c682a97-5d98-49c9-a5f7-b93ec7b036f7/specs/proposal-review-contract/spec.md`, `.devagent/docs/docs/proposals.md`, `.devagent/docs/docs/production-suite.md`, `.devagent/docs/docs/cli-v0-contract.md`
