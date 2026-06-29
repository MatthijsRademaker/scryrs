# cli-doctor-command Specification

## Purpose
TBD - created by archiving change task-782aa74d-a495-4a24-b877-ae014b85137b. Update Purpose after archive.
## Requirements
### Requirement: `scryrs doctor` is a native root CLI command with human and JSON output

The shipped CLI SHALL expose `scryrs doctor` as a native root command. The default output SHALL be human-readable for installed users. `scryrs doctor --json` SHALL emit the same diagnostic categories in machine-readable form for automation. The root help and machine-readable help surface SHALL include the command.

#### Scenario: Default doctor invocation prints a human-readable diagnosis

- **GIVEN** a user has the shipped `scryrs` binary installed
- **WHEN** they run `scryrs doctor`
- **THEN** the command exits with a documented status code
- **AND** stdout contains a human-readable diagnostic summary
- **AND** the summary includes binary version, resolved mode, installation findings, and docs links

#### Scenario: JSON mode exposes the same diagnostic categories for automation

- **GIVEN** an installed `scryrs` binary
- **WHEN** a maintainer runs `scryrs doctor --json`
- **THEN** stdout is valid JSON
- **AND** the JSON includes the binary version
- **AND** the JSON includes the shipped CLI surface / feature availability
- **AND** the JSON includes the resolved mode
- **AND** the JSON includes per-check findings for store, hooks, and server reachability when applicable
- **AND** the JSON includes docs links

#### Scenario: Doctor is discoverable through CLI help surfaces

- **WHEN** a user runs `scryrs --help` or `scryrs --help-json`
- **THEN** the root command surface includes `doctor`
- **AND** the help text describes it as the installation and readiness diagnostic command

### Requirement: Doctor reports binary surface, resolved mode, store status, hook status, server reachability, and docs links

`scryrs doctor` SHALL report the minimum diagnostic set required by the production-hardening task:

- binary version
- shipped command surface / feature availability
- resolved mode as `local` or `live`
- local store status
- Claude Code hook status where applicable
- Pi hook status where applicable
- live server reachability when live mode is configured
- relevant docs links

Mode resolution SHALL use the same local-vs-live configuration logic as record transport. The command SHALL NOT silently fall back from live to local.

#### Scenario: Local mode is reported clearly

- **GIVEN** a repository without live ingest configuration
- **WHEN** `scryrs doctor` runs successfully
- **THEN** the result reports `mode: local`
- **AND** the output distinguishes local store findings from any live-server finding
- **AND** the command does not imply that local store health proves live ingest health

#### Scenario: Live mode is reported clearly and does not silently fall back

- **GIVEN** live ingest configuration is present
- **WHEN** `scryrs doctor` runs
- **THEN** the result reports `mode: live`
- **AND** the command evaluates configured live-server reachability
- **AND** it does not downgrade the diagnosis to local mode when the live server is unreachable

#### Scenario: Hook findings distinguish Claude Code and Pi independently

- **GIVEN** one supported hook is installed and the other is not
- **WHEN** `scryrs doctor` runs
- **THEN** the output reports Claude Code and Pi hook status as separate findings
- **AND** each finding explains the detected state without conflating the two harnesses

### Requirement: Doctor uses `ok`, `warn`, and `error` findings with exit 2 for structural failures

Each doctor check SHALL produce an `ok`, `warn`, or `error` finding. `scryrs doctor` SHALL exit `0` when all findings are `ok` or `warn`. It SHALL exit `2` when any structural `error` is present.

Structural errors include unusable configured live mode, corrupt or unreadable configuration or store state, and other conditions that make the diagnosed setup non-operational. Advisory conditions such as an uninitialized local workspace or a missing optional hook SHALL be warnings unless they prevent the configured mode from functioning.

#### Scenario: Advisory findings do not fail the command

- **GIVEN** a repository has no initialized local store or is missing one optional hook
- **WHEN** `scryrs doctor` completes with only advisory findings
- **THEN** the command exits `0`
- **AND** the findings are reported as `warn`
- **AND** the output explains the recommended next step

#### Scenario: Unusable configured live mode fails the command

- **GIVEN** live mode is configured but the remote identity is incomplete or the configured live server is unreachable
- **WHEN** `scryrs doctor` runs
- **THEN** the command exits `2`
- **AND** the failing finding is reported as `error`
- **AND** the output makes clear that live mode did not fall back to local mode

#### Scenario: Corrupt configuration or unreadable store fails the command

- **GIVEN** the workspace contains corrupt configuration or an unreadable store state
- **WHEN** `scryrs doctor` runs
- **THEN** the command exits `2`
- **AND** the output identifies the corrupt or unreadable component as an `error`

### Requirement: Doctor coverage includes the production-hardening diagnosis matrix

The implementation SHALL include automated coverage for the diagnostic states called out during refinement: no config/local empty store, local initialized store, live config with unreachable server, missing remote identity, Claude Code hook present, Pi hook present, and unsupported or corrupt config reporting. The CLI reference SHALL document the doctor command contract, including output categories and exit-code behavior.

#### Scenario: No-config and initialized-local cases are both covered

- **WHEN** the doctor test suite runs
- **THEN** it exercises a no-config or empty-local-state case
- **AND** it exercises a healthy initialized local workspace case
- **AND** the assertions distinguish warning-only diagnosis from healthy diagnosis

#### Scenario: Live-failure cases are covered

- **WHEN** the doctor test suite runs
- **THEN** it exercises a live configuration with an unreachable server
- **AND** it exercises a missing remote-identity case
- **AND** both cases assert `error` findings and exit `2`

#### Scenario: Hook-detection and corrupt-config cases are covered

- **WHEN** the doctor test suite runs
- **THEN** it exercises Claude Code hook detection
- **AND** it exercises Pi hook detection
- **AND** it exercises unsupported or corrupt configuration reporting
- **AND** the documented CLI contract matches the exercised behavior

