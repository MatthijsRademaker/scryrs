## Context

`scryrs init` now defaults to live mode and already has a complete live bootstrap contract: resolve `ingest_url`, `workspace_id`, and `docker_network`; validate before writes; write committed shared values into `scryrs.json`; scaffold managed `.scryrs/` live files; install the requested harness hook. When those values are absent, the current CLI fails fast with deterministic guidance. That is correct for automation, but rough for an interactive first run because the missing values are exactly the values a wizard can collect.

The change must preserve the existing precedence chain and fail-fast behavior for non-interactive execution. The wizard is a user-interface layer over the existing live-init model, not a new config source or transport path.

## Goals / Non-Goals

**Goals:**

- Make `scryrs init --agent <NAME>` usable as a guided live-mode first-run command in an interactive terminal.
- Add `--no-interactive` as an explicit opt-out that preserves promptless validation and deterministic exit-2 errors.
- Keep scripts, CI, pipes, and tests from hanging by detecting terminal availability before prompting.
- Provide richer wizard UX than raw `read_line`: clear intro text, per-field help, defaults where available, validation, retry, and final confirmation.
- Preserve existing live-mode output artifacts and ownership rules.

**Non-Goals:**

- No local-mode wizard.
- No new persisted config fields beyond existing `scryrs.json remote` keys.
- No change to hook behavior, remote submission, live server API, record mode, trace event schema, or dashboard behavior.
- No automatic Docker network creation or live server startup from the wizard.
- No `--interactive` flag unless implementation discovers an unavoidable need; default human path is interactive when safe.

## Decisions

### Decision 1: Prompt only when live init is missing required config and terminal IO is interactive

`execute_init` should first resolve flags/env/.scryrs/existing manifest as it does today. If live-mode required fields are complete, it proceeds without prompts. If fields are missing, `--no-interactive` or non-terminal stdin/stdout keeps the current exit-2 guidance. Only when missing fields exist and both input and output are terminals does the wizard run.

Alternatives considered:

- Always prompt by default: rejected because it can hang automation.
- Require `--interactive`: rejected because it fails the desired default first-run UX.
- Prompt even when all fields resolve: rejected because it adds noisy confirmation to already deterministic invocations.

### Decision 2: Add `--no-interactive` only

`--no-interactive` is the explicit automation/opt-out control. It should be exposed in clap dispatch, help text, and help-json for `init`. Tests should assert that `--no-interactive` plus missing live fields exits 2 without prompting.

Alternatives considered:

- Add both `--interactive` and `--no-interactive`: rejected for now because `--interactive` creates unnecessary precedence questions and is not needed for requested behavior.
- Environment-only opt-out: rejected because explicit CLI behavior is easier to discover and test.

### Decision 3: Use a prompt backend behind a small internal trait

Rich UX should be implemented behind a narrow internal interface, e.g. `InitPrompt`, so tests can supply deterministic answers without a real TTY. The production implementation may use a focused Rust prompt crate such as `dialoguer` for text prompts, confirmation, validation loops, and styling. The dependency should stay confined to `scryrs-cli`.

Alternatives considered:

- Hand-roll prompts with `stdin.read_line`: rejected because user explicitly wants rich wizard UX and validation/retry ergonomics.
- Embed prompt logic directly into `execute_init`: rejected because tests would become brittle and terminal behavior hard to simulate.

### Decision 4: Wizard collects missing required fields, shows resolved defaults, then confirms

The wizard should explain that live init writes committed shared config to `scryrs.json remote` and local overrides to `.scryrs/.env` only as an empty stub. It should prompt for missing `ingest_url`, `workspace_id`, and `docker_network`. If an existing layer already supplied a value, the wizard may show it in the summary but should not force re-entry. Before writes, it shows the final values and asks for confirmation. Cancellation exits 2 and writes nothing.

Validation rules should reuse or mirror existing live validation: non-empty fields, usable URL for `ingest_url`, non-empty IDs, and non-empty Docker network name. `repository_id` remains derived/overridden by existing resolution; `agent_id` remains optional and not committed.

Alternatives considered:

- Prompt for every field every time: rejected because flags/env/manifests should remain authoritative.
- Write answers incrementally: rejected because init must avoid partial live bootstrap writes.

### Decision 5: Keep live bootstrap artifacts unchanged after wizard completion

After the wizard returns a complete live config, the existing validation, manifest write, `.scryrs/` scaffold, compose scaffold, and hook install paths should run. This minimizes risk and preserves the current contract for `scryrs up`, hooks, and documentation.

Alternatives considered:

- Add wizard-specific manifest writer: rejected as duplicate business logic.
- Store wizard answers in `.scryrs/.env`: rejected because committed `scryrs.json remote` remains source of truth for shared live config.

## Risks / Trade-offs

| Risk | Mitigation |
| --- | --- |
| Interactive prompts hang scripts or CI. | Gate prompts on terminal detection and `--no-interactive`; keep non-terminal behavior fail-fast. |
| Prompt crate complicates dependency tree or binary size. | Choose a small CLI-focused dependency and confine it to `scryrs-cli`; keep prompt backend swappable. |
| Tests become hard because terminal detection is environmental. | Inject prompt/terminal state behind testable interfaces and add no-interactive regression tests. |
| Wizard wording misleads users about committed vs local config. | Include explicit intro and confirmation summary naming `scryrs.json remote` and `.scryrs/.env` roles. |
| Cancellation after some prompts leaves partial files. | Run wizard before filesystem writes and treat cancellation as exit 2. |

## Migration Plan

- Add `--no-interactive` to `scryrs init`; existing flag-driven and env-driven invocations continue to work unchanged.
- Interactive users with missing live config see the wizard instead of the old missing-field error.
- Automation that wants the current behavior can pass `--no-interactive`; non-TTY automation gets current fail-fast behavior even without the flag.
- Rollback is removing wizard invocation and `--no-interactive` dispatch surface; no persisted data migration is involved.

## Open Questions

- Which prompt crate should be accepted after checking dependency footprint (`dialoguer` is the leading candidate)?
- Should wizard defaults propose `http://scryrs:8081` and `scryrs-net`, or avoid opinionated defaults and require explicit entry? The safer initial design is no invented defaults unless an existing resolution layer supplies one.
