## 1. CLI Surface And Prompt Infrastructure

- [x] 1.1 Add `--no-interactive` to `scryrs init` dispatch parsing and thread it into init execution.
- [x] 1.2 Update help text and help-json so `init` documents `--no-interactive`, TTY-only wizard behavior, and promptless fail-fast behavior.
- [x] 1.3 Add a small init prompt abstraction that can run a production rich prompt backend and deterministic test fakes.
- [x] 1.4 Choose and add the rich prompt dependency for `scryrs-cli`, or document and implement an equivalent internal prompt backend if dependency review rejects it.
- [x] 1.5 Add terminal detection using stdin/stdout terminal state so prompt eligibility is testable and does not depend on ambient test process IO.

## 2. Live Init Wizard Behavior

- [x] 2.1 Refactor live-config resolution so missing required fields can be represented before emitting the current missing-config error.
- [x] 2.2 Invoke the wizard only for live mode when required fields are missing, terminal IO is available, and `--no-interactive` is absent.
- [x] 2.3 Prompt for missing `ingest_url`, `workspace_id`, and `docker_network` while preserving already-resolved values from flags, env, `.scryrs/.env`, or `scryrs.json`.
- [x] 2.4 Validate wizard input before any filesystem writes and reject empty required values.
- [x] 2.5 Show a final confirmation summary that names the committed `scryrs.json remote` fields and the overrides-only `.scryrs/.env` role.
- [x] 2.6 Treat wizard cancellation or confirmation rejection as exit code 2 with no hook files, `.scryrs/` artifacts, or `scryrs.json` changes.
- [x] 2.7 After confirmed wizard input, reuse the existing live manifest, `.scryrs/`, compose, and harness install paths without duplicating write logic.

## 3. Tests

- [x] 3.1 Add init tests proving `--no-interactive` plus missing live config exits 2 without prompt output or partial writes.
- [x] 3.2 Add init tests proving non-terminal stdin or stdout plus missing live config exits 2 without blocking or partial writes.
- [x] 3.3 Add init tests proving interactive missing config uses fake prompt answers and successfully writes `scryrs.json remote`, `.scryrs/.env`, `.scryrs/compose.yml`, and the requested harness hook.
- [x] 3.4 Add init tests proving resolved fields are not re-prompted and still appear in the confirmation summary.
- [x] 3.5 Add init tests proving cancellation before confirmation exits 2 and leaves no partial files.
- [x] 3.6 Add dispatch/help-json tests proving `--no-interactive` is accepted and exposed in machine-readable help.
- [x] 3.7 Run focused init/dispatch tests before broader workspace verification.

## 4. Documentation And Verification

- [x] 4.1 Update README live-init guidance with interactive wizard behavior, `--no-interactive`, and examples for both wizard and flag-driven setup.
- [x] 4.2 Update project CLI docs or generated CLI reference sources so docs match `scryrs --help` and `--help-json`.
- [x] 4.3 Verify OpenSpec requirements are satisfied by tests or documented manual checks.
- [x] 4.4 Run `cargo test -p scryrs-cli` and fix any regressions.
- [x] 4.5 Run `openspec status --change add-init-interactive-wizard` and ensure the change remains apply-ready.
