# cli-binary-installer Specification

## Purpose
TBD - created by archiving change task-f85aaf05-e57c-4029-b168-0275448029b4. Update Purpose after archive.
## Requirements
### Requirement: Source-checkout installer script exists for supported Unix platforms

The repository SHALL provide an executable Bash installer at `scripts/install` that installs the `scryrs` CLI from a source checkout on macOS and Linux only.

#### Scenario: Supported operating systems proceed

- **WHEN** `scripts/install` runs on a system where `uname` returns `Linux` or `Darwin`
- **THEN** the installer continues with the install flow

#### Scenario: Unsupported operating systems fail without mutation

- **WHEN** `scripts/install` runs on a system where `uname` returns any other value
- **THEN** the installer exits non-zero with a clear unsupported-platform message
- **AND** no install directory or binary is created or modified

### Requirement: Install directory selection is configurable and deterministic

The installer SHALL target `$HOME/.local/bin` by default and SHALL allow the install directory to be supplied via `--bin-dir <PATH>` or `SCRYRS_INSTALL_DIR`.

#### Scenario: Default install directory is used

- **WHEN** the installer runs without `--bin-dir` and without `SCRYRS_INSTALL_DIR`
- **THEN** the target install directory is `$HOME/.local/bin`

#### Scenario: Command-line directory override is used

- **WHEN** the installer runs with `--bin-dir /tmp/scryrs-bin`
- **THEN** the target install directory is `/tmp/scryrs-bin`

#### Scenario: Environment directory override is used

- **WHEN** the installer runs with `SCRYRS_INSTALL_DIR=/tmp/scryrs-bin` and no `--bin-dir`
- **THEN** the target install directory is `/tmp/scryrs-bin`

### Requirement: Installer builds, installs, and verifies the default CLI binary

The installer SHALL build `scryrs-cli` from the repository root with `cargo build -p scryrs-cli --release --locked` using the default feature set, copy `target/release/scryrs` into the chosen install directory as `scryrs`, ensure it is executable, overwrite any existing `scryrs` at that target path as idempotent upgrade behavior, and verify the installed binary by running `<install-dir>/scryrs --version`.

#### Scenario: Fresh install succeeds

- **GIVEN** Rust/Cargo is available and the repository contains the `scryrs-cli` crate
- **WHEN** the installer runs
- **THEN** `cargo build -p scryrs-cli --release --locked` is executed
- **AND** `<install-dir>/scryrs` exists and is executable
- **AND** `<install-dir>/scryrs --version` exits 0

#### Scenario: Existing binary is upgraded in place

- **GIVEN** `<install-dir>/scryrs` already exists
- **WHEN** the installer runs again
- **THEN** the binary at `<install-dir>/scryrs` is replaced with the newly built binary
- **AND** unrelated files in `<install-dir>` are unchanged

#### Scenario: Verification failure fails loudly

- **WHEN** the installed binary cannot execute `--version` successfully
- **THEN** the installer exits non-zero
- **AND** the failure is reported clearly

### Requirement: Installer reports PATH usability without editing shell startup files

The installer SHALL check whether the chosen install directory is already on `PATH`. If it is, `command -v scryrs` within the installer process SHALL resolve to the installed binary. If it is not, the installer SHALL print exact PATH guidance and SHALL NOT edit shell startup files.

#### Scenario: Install directory already on PATH

- **GIVEN** the chosen install directory is present on `PATH`
- **WHEN** installation succeeds
- **THEN** `command -v scryrs` resolves to the installed binary in that process

#### Scenario: Install directory missing from PATH

- **GIVEN** the chosen install directory is not present on `PATH`
- **WHEN** installation succeeds
- **THEN** the installer prints exact PATH update guidance
- **AND** no shell startup file is modified

### Requirement: Binary installation remains separate from hook and user config installation

The installer SHALL only manage the `scryrs` executable and SHALL NOT create or modify `.claude/`, `.pi/`, `.scryrs/`, `scryrs.json`, git hooks, or shell profile files. `README.md` SHALL clarify that `scryrs init --agent <NAME>` installs hooks only after `scryrs` is on `PATH`.

#### Scenario: Installer does not touch hook or config artifacts

- **WHEN** `scripts/install` completes
- **THEN** no files are created or modified under `.claude/`, `.pi/`, `.scryrs/`, or git hook paths
- **AND** no `scryrs.json` or shell profile file is created or edited

#### Scenario: README distinguishes binary install from hook install

- **WHEN** a user reads the install instructions in `README.md`
- **THEN** the document includes a copy-paste install-from-source flow for macOS/Linux
- **AND** it states that `scryrs init --agent claude-code` and `scryrs init --agent pi` install hooks after the `scryrs` binary is already on `PATH`

### Requirement: Automated verification covers installer syntax and Linux temp-directory install

The repository SHALL provide a `scripts/verify-install` verification path that checks installer shell syntax and exercises a Linux temp-directory installation using the existing Docker-backed Rust verification helpers.

#### Scenario: Shell syntax is verified

- **WHEN** installer verification runs
- **THEN** it executes `bash -n scripts/install`

#### Scenario: Linux temp-directory install is verified

- **WHEN** installer verification runs through `run_rust` from `scripts/lib/docker-verification.sh`
- **THEN** it installs `scryrs` into a temporary directory
- **AND** it verifies the installed binary with `<temp-install-dir>/scryrs --version`

