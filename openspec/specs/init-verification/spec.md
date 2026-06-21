# init-verification Specification

## Purpose
TBD - created by archiving change task-fd4b6e09-3d40-4900-b824-7637aef8899d. Update Purpose after archive.
## Requirements
### Requirement: Installed-hook e2e is wired into the authoritative verification lane

`scripts/verify-trace-capture` SHALL include `scripts/verification/installed-hook-e2e.mjs` as a fixture phase. The installed-hook fixture SHALL be exercisable both as part of the full lane (after source-hook fixtures) and independently via an `--init-only` flag. The fixture SHALL use the same Docker image (`FIXTURE_NODE_IMAGE`, default `node:22`) and scryrs binary as the existing source-hook fixtures.

#### Scenario: Full lane runs all three fixtures

- **GIVEN** Docker is available and `cargo build --release` has completed
- **WHEN** `scripts/verify-trace-capture` is invoked without flags
- **THEN** the Claude Code source-hook fixture (`claude-code-e2e.mjs`) executes
- **AND** the Pi source-hook fixture (`pi-hook-e2e.mjs`) executes
- **AND** the installed-hook e2e fixture (`installed-hook-e2e.mjs`) executes
- **AND** the lane exits 0 if all fixtures pass

#### Scenario: --init-only runs installed-hook e2e independently

- **GIVEN** Docker is available and `cargo build --release` has completed
- **WHEN** `scripts/verify-trace-capture --init-only` is invoked
- **THEN** only the installed-hook e2e fixture (`installed-hook-e2e.mjs`) executes
- **AND** source-hook fixtures (`claude-code-e2e.mjs`, `pi-hook-e2e.mjs`) are skipped
- **AND** the lane exits 0 if the installed-hook fixture passes

#### Scenario: --init-only composes with existing filter flags

- **WHEN** `scripts/verify-trace-capture --init-only --claude-only` is invoked
- **THEN** the script exits 2 with a usage error indicating the flags are mutually exclusive

#### Scenario: Installed-hook failure fails the lane

- **GIVEN** the installed-hook e2e fixture encounters a failure (e.g., hook file not loadable)
- **WHEN** `scripts/verify-trace-capture` is invoked (full lane or `--init-only`)
- **THEN** the lane exits non-zero
- **AND** the summary output reports which fixture failed

### Requirement: Installed-hook e2e validates consumer-installed artifacts

The installed-hook e2e fixture SHALL run `scryrs init --agent claude-code` and `scryrs init --agent pi` in temporary consumer project directories, load the installed hook artifacts from their consumer install paths (NOT from `hooks/` in the repository source tree), exercise tool-capture forwarding against the real `scryrs` binary, and prove at least one event is persisted in `.scryrs/scryrs.db`.

#### Scenario: Claude Code installed hook is loadable and functional

- **GIVEN** a temporary consumer project directory
- **WHEN** `scryrs init --agent claude-code` is executed in that directory
- **THEN** the installed `scryrs-hook.mjs` file exists at `.claude/hooks/scryrs-hook.mjs`
- **AND** the installed file can be loaded as a Node.js module without import errors
- **AND** when exercised with a valid tool input, the hook forwards the event to `scryrs record --stdin`
- **AND** event persistence is confirmed via `scryrs hotspots .` showing `analyzedEventCount >= 1`

#### Scenario: Pi installed hook is loadable and functional

- **GIVEN** a temporary consumer project directory
- **WHEN** `scryrs init --agent pi` is executed in that directory
- **THEN** the installed Pi hook artifact exists at `.pi/extensions/pi-trace/index.ts`
- **AND** the installed artifact can be loaded (TypeScript transpiled via tsx) without errors
- **AND** when exercised with a simulated `tool_result` event, the hook forwards the event to `scryrs record --stdin`
- **AND** event persistence is confirmed via `scryrs hotspots .` showing `analyzedEventCount >= 1`

#### Scenario: Installed hook e2e does not depend on repository source

- **GIVEN** the installed-hook e2e fixture is executing
- **WHEN** the fixture loads hook artifacts
- **THEN** artifacts are loaded from the temporary consumer project directory (`.claude/hooks/` or `.pi/extensions/pi-trace/`)
- **AND** artifacts are NOT loaded from `hooks/claude-code/` or `hooks/pi/` in the repository source tree

#### Scenario: Failed init is detected by the e2e fixture

- **GIVEN** `scryrs init --agent claude-code` produces a corrupt or unloadable hook file
- **WHEN** the installed-hook e2e fixture attempts to load the installed artifact
- **THEN** the fixture fails with a diagnostic message indicating which artifact could not be loaded
- **AND** the fixture does not silently skip the load step

### Requirement: Installed-hook e2e validates deterministic next-step text

The installed-hook e2e fixture SHALL assert that the stdout output from a successful `scryrs init` invocation includes the deterministic next-step text from the harness registry. For Claude Code, the next-step text SHALL instruct the user to create `.claude/settings.json` manually (the installer does not auto-create it). For Pi, the next-step text SHALL instruct the user to reload Pi.

#### Scenario: Claude Code next-step text instructs manual settings.json creation

- **WHEN** `scryrs init --agent claude-code` completes successfully
- **THEN** stdout includes text instructing the user to manually create `.claude/settings.json`
- **AND** stdout includes the hook configuration that the user must insert
- **AND** stdout includes restart or reload instructions

#### Scenario: Pi next-step text instructs reload

- **WHEN** `scryrs init --agent pi` completes successfully
- **THEN** stdout includes text instructing the user to reload Pi
- **AND** stdout notes that scryrs must be on PATH

#### Scenario: Next-step text is deterministic across invocations

- **WHEN** `scryrs init --agent claude-code` is invoked twice in separate temporary directories
- **THEN** the stdout output is byte-identical for both invocations

### Requirement: Claude Code settings.json schema is consistent across all sources

The canonical `.claude/settings.json` hook configuration schema SHALL be identical across the installer next-steps text and the hook README documentation. Users following either source SHALL produce the same `.claude/settings.json` configuration.

#### Scenario: Installer next-steps matches hook README schema

- **GIVEN** the canonical schema form has been chosen (flat `"hook"` string or nested `"type":"command"` command-block)
- **WHEN** comparing the schema emitted by `scryrs init --agent claude-code` next-steps text
- **AND** comparing the schema documented in `hooks/claude-code/README.md`
- **THEN** both sources describe the same JSON structure for registering the scryrs hook

#### Scenario: Collision error JSON block matches canonical schema

- **GIVEN** `.claude/settings.json` already exists
- **WHEN** `scryrs init --agent claude-code` is invoked and exits 2 with a collision error
- **THEN** the JSON block printed in the collision error matches the canonical schema form
- **AND** the collision error JSON block matches the next-steps JSON block for the successful-install path

### Requirement: Verification README documents all fixtures

`scripts/verification/README.md` SHALL list `installed-hook-e2e.mjs` in its fixture tree alongside `claude-code-e2e.mjs` and `pi-hook-e2e.mjs`. The documentation SHALL describe what the installed-hook e2e fixture proves and how it differs from the source-hook fixtures.

#### Scenario: Fixture tree includes installed-hook e2e

- **WHEN** reading the Architecture section of `scripts/verification/README.md`
- **THEN** the fixture tree lists `installed-hook-e2e.mjs`
- **AND** the tree also lists `claude-code-e2e.mjs` and `pi-hook-e2e.mjs`
- **AND** each fixture has a description of its purpose

#### Scenario: Installed-hook e2e purpose is documented

- **WHEN** reading the description of `installed-hook-e2e.mjs` in the README
- **THEN** the description states that it proves init output produces functional consumer-installed artifacts
- **AND** the description distinguishes it from source-hook fixtures (which load from `hooks/` in the source tree)

### Requirement: Init installer contract remains non-mutating for settings.json

The init installer SHALL NOT be changed to auto-create, modify, or overwrite `.claude/settings.json`. The existing contract (refuse when settings.json exists, instruct user to create it when absent) SHALL remain intact. This requirement is already specified in `init-installer/spec.md` and is reasserted here to confirm no regression.

#### Scenario: settings.json is never auto-created

- **GIVEN** `.claude/settings.json` does not exist
- **WHEN** `scryrs init --agent claude-code` completes successfully
- **THEN** `.claude/settings.json` does not exist after the command completes

#### Scenario: settings.json collision still exits 2

- **GIVEN** `.claude/settings.json` already exists
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the command exits 2
- **AND** the existing file is not modified

### Requirement: Pi version-gated assumption is documented

The installed-hook e2e fixture SHALL include a comment documenting the Pi version(s) for which single-file `index.ts` sufficiency has been verified. If Pi extension contract changes (requiring additional manifest or config artifacts), the comment SHALL serve as a gating notice requiring the test to be updated.

#### Scenario: Pi version assumption is documented in e2e script

- **WHEN** reading `scripts/verification/installed-hook-e2e.mjs`
- **THEN** the file contains a comment stating which Pi version(s) the single-file `index.ts` assumption was verified against
- **AND** the comment notes that if Pi requires additional artifacts (manifest, package.json, tsconfig), the test must be updated

