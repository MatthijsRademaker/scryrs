## Why

The Pi trace hook and `scryrs record --stdin` currently form a mostly opaque pipeline: failures are fail-open by design, but that makes hook loading, event observation, field mapping, subprocess execution, ingestion rejection, and persistence failures hard to diagnose without guessing. Developers need opt-in debug breadcrumbs to verify the end-to-end trace path and inspect what Pi actually emits on the wire, especially around tool names, input field names, RTK-style command rewrites, and schema rejection cases.

## What Changes

- Add opt-in debug logging controlled by `SCRYRS_DEBUG` for the canonical Pi hook in `hooks/pi/index.ts`.
- Add opt-in debug logging controlled by `SCRYRS_DEBUG` for `scryrs record --stdin` and `scryrs record --file <PATH>` ingestion.
- Log hook lifecycle and pipeline stages: hook load, `session_start`, `tool_result`, tracked/untracked tool decision, input keys, field fallback, record subprocess send/result, non-zero exits, and exec errors.
- Log record-side ingestion stages: line received, accepted event summary, rejected line diagnostics, datastore open, inserted accepted event summary, and final summary.
- Keep debug logs out of normal operation. With `SCRYRS_DEBUG` unset, existing stdout/stderr contracts, exit codes, persistence, fail-open behavior, and non-interference remain unchanged.
- Add a wire-inspection mode for sanitized observed inputs without dumping full tool results or file contents by default.
- Extend verification scripts so debug output can be asserted as pipeline evidence in addition to database rows.
- Document the debug flag and safe usage in Pi hook and/or verification documentation.
- Refresh the installed local Pi artifact by removing `.pi/extensions/pi-trace/index.ts` and rerunning `scryrs init --agent pi`; do not edit the installed copy directly.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `pi-reference-hook`: Add opt-in debug observability for hook loading, lifecycle/tool events, field extraction, subprocess execution, and fail-open diagnostics without changing trace capture semantics.
- `scryrs-record-endpoint`: Add opt-in debug observability for JSONL ingestion, validation acceptance/rejection, datastore persistence, and record summary without changing normal stdout/stderr or exit-code contracts.
- `init-verification`: Extend trace-capture verification to assert debug breadcrumbs for the Pi hook and record pipeline where useful.

## Impact

- `hooks/pi/index.ts`: canonical Pi hook debug logging and wire-inspection behavior.
- `crates/scryrs-cli/src/record.rs`: record-side debug logging gated by `SCRYRS_DEBUG`.
- `scripts/verification/pi-hook-e2e.mjs`, `scripts/verification/installed-hook-e2e.mjs`, and/or `scripts/verify-trace-capture`: assertions for debug breadcrumbs.
- `hooks/pi/README.md` and/or `scripts/verification/README.md`: developer-facing debug usage notes.
- `.pi/extensions/pi-trace/index.ts`: refreshed generated local runtime artifact only via `scryrs init --agent pi`; not edited directly.
- No database schema changes, no new dependencies, no change to default CLI output, no change to normal hook fail-open behavior, and no task-list changes outside this OpenSpec proposal.
