## 1. Add record debug tests and helpers

- [x] 1.1 Add `scryrs record` tests proving `SCRYRS_DEBUG` unset emits no `[scryrs-record]` lines and preserves current stdout/stderr contracts.
- [x] 1.2 Add `scryrs record` tests proving `SCRYRS_DEBUG=1` emits `[scryrs-record]` lines for received, accepted, rejected, inserted, and summary stages.
- [x] 1.3 Implement record-side debug helpers in `crates/scryrs-cli/src/record.rs` for env detection, newline collapse, truncation, and prefixed stderr writes.
- [x] 1.4 Wire record-side debug logging into stdin/file input receipt without changing accepted/rejected decisions or normal output when debug is disabled.
- [x] 1.5 Wire record-side debug logging into validation outcome, datastore open, event insertion, transaction commit, and final exit summary.

## 2. Add Pi hook debug tests and helpers

- [x] 2.1 Extend Pi hook verification or hook-level tests to capture console output with `SCRYRS_DEBUG` unset and prove no `[scryrs]` debug lines are emitted.
- [x] 2.2 Add Pi hook debug tests for hook load, `session_start`, tracked `tool_result`, untracked `tool_result`, missing mapped field, record subprocess result, and exec failure breadcrumbs.
- [x] 2.3 Implement hook-side debug helpers in `hooks/pi/index.ts` for env detection, debug mode selection, input key extraction, newline collapse, truncation, and stable `[scryrs]` lines.
- [x] 2.4 Add hook-load and `session_start` debug breadcrumbs without blocking session startup.
- [x] 2.5 Add `tool_result` debug breadcrumbs for every observed tool, including tracked/untracked decision and input keys.
- [x] 2.6 Add missing-field debug breadcrumbs that include wanted field, available keys, and fallback value while preserving existing fallback event mapping.
- [x] 2.7 Add `recordEvent` debug breadcrumbs for send attempt, subprocess result code/killed status, truncated stdout/stderr previews, non-zero exits, and exec errors.
- [x] 2.8 Add `SCRYRS_DEBUG=wire` sanitized input previews and bounded raw/debug preview behavior without dumping full `content` or `details` by default.

## 3. Extend verification fixtures

- [x] 3.1 Update `scripts/verification/pi-hook-e2e.mjs` to run a debug-enabled path and assert required `[scryrs]` and echoed `[scryrs-record]` breadcrumbs by presence, not exact ordering.
- [x] 3.2 Add Pi source-hook fixture coverage for untracked-tool debug logging and no persisted row for the untracked tool.
- [x] 3.3 Add Pi source-hook fixture coverage for missing-field debug logging and fallback persistence.
- [x] 3.4 Update `scripts/verification/installed-hook-e2e.mjs` to verify the installed Pi hook emits debug breadcrumbs when loaded from `.pi/extensions/pi-trace/index.ts`.
- [x] 3.5 Ensure existing `scripts/verify-trace-capture` modes still pass with debug disabled by default.

## 4. Documentation and installed artifact refresh

- [x] 4.1 Document `SCRYRS_DEBUG=1`, `SCRYRS_DEBUG=wire`, and any raw mode guidance in `hooks/pi/README.md` and/or `scripts/verification/README.md`.
- [x] 4.2 State that debug logs are opt-in, bounded, and intended for development; warn that wire/raw modes may expose observed tool inputs.
- [x] 4.3 Remove `.pi/extensions/pi-trace/index.ts` and rerun `scryrs init --agent pi` after editing `hooks/pi/index.ts`; do not directly patch the installed artifact.
- [x] 4.4 Verify the refreshed installed artifact content matches the canonical embedded hook source produced by `scryrs init --agent pi`.

## 5. Validation

- [x] 5.1 Run `cargo test -p scryrs-cli record` or the focused record test subset covering debug behavior.
- [x] 5.2 Run TypeScript/LSP diagnostics for `hooks/pi/index.ts`.
- [x] 5.3 Run `scripts/verification/pi-hook-e2e.mjs` or `scripts/verify-trace-capture --pi-only` as environment allows.
- [x] 5.4 Run `scripts/verify-trace-capture --init-only` or equivalent installed-hook verification as environment allows.
- [x] 5.5 Run `openspec status --change add-scryrs-debug-logging` and confirm all implementation tasks are tracked.
