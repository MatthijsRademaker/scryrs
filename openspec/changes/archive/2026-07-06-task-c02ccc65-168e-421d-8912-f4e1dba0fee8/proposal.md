## Why

Reviewers can generate and review proposal artifacts via the file-based proposal workflow (`scryrs proposals list`, `accept`, `reject`), but the dashboard currently exposes only hotspots, subjects, sessions, events, signals, and meta. Proposal inbox state therefore remains trapped in raw JSON files under `.scryrs/proposals/`, `.scryrs/accepted/`, and `.scryrs/rejected/`, even though deterministic listing and validation semantics already exist in the CLI.

This change adds a read-only proposal inbox surface to the dashboard so reviewers can browse proposal state and inspect details without opening raw JSON files. The dashboard view mirrors the exact same state-derivation and validation rules as `scryrs proposals list`, fails loudly on malformed artifacts, and introduces no write path.

## What Changes

1. **Extract proposal inventory logic into `scryrs-curator`**: The file-based proposal loading, validation, and state-derivation functions (`collect_list_rows`, `load_proposals`, `load_review_decisions`, validators) are extracted from `scryrs-cli/src/proposals.rs` (currently `pub(crate)`) into a new public module in `scryrs-curator` so both the CLI and dashboard consume identical deterministic semantics. CLI-specific `CommandError` types are replaced with a shared error type that each consumer maps to its own error representation.

2. **Add local-only dashboard API endpoints**: `GET /api/proposals` returns deterministic rows (proposalId, title, targetType, createdAt, state) sorted by proposalId ascending. `GET /api/proposals/:proposalId` returns the full `ProposalDocument` plus optional `ProposalReviewDecision` metadata for the detail view. Both endpoints return JSON 404 when `.scryrs/proposals/` is missing and JSON 502 for malformed/conflicting artifacts, following the existing `missing_files_return_json_404_and_corrupt_store_returns_502` test pattern.

3. **Add Proposals frontend view**: Two new routes (`/proposals` for the list, `/proposals/:proposalId` for detail), a Pinia store following the sessions store pattern, local-only navigation gating with an unavailable message in live mode, and read-only rendering of proposal rationale, proposed content, evidence links, and optional review decision metadata. Destructive Alert surfaces API errors; `<pre>` blocks render proposal content (deferring rich markdown rendering to a follow-up).

4. **Extend docs and verification**: The dashboard API contract docs and the architecture crate map are updated to reflect the new endpoint and curator module. Automated coverage is added for backend endpoint behavior and frontend navigation, data, and error-rendering paths.

## Impact

- **`scryrs-curator`** gains a new public module (`proposals::inventory`) with filesystem I/O dependency. This is the first curator module to read from disk; prior curator logic was pure proposal generation.
- **`scryrs-dashboard`** gains a production dependency on `scryrs-curator` and registers two new routes. Backend error patterns follow the existing `ApiError` convention (404 for missing, 502 for malformed).
- **`scryrs-cli`** re-exports from `scryrs-curator::proposals::inventory` rather than owning the proposal read logic directly. Existing CLI tests for deterministic listing, conflicting terminal states, and invalid artifact failures remain the executable spec.
- **Frontend** gains two new Vue routes, a new Pinia store, and new API client DTOs. Navigation, local/live gating, and error handling follow the established Sessions pattern.
- **No mutations**: No accept/reject actions, no proposal edits, and no writes to `.scryrs/proposals/`, `.scryrs/accepted/`, `.scryrs/rejected/`, `.scryrs/graph.json`, or any other persistent state.