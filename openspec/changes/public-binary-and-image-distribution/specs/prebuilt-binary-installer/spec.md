## ADDED Requirements

### Requirement: One-shot installer downloads a published binary without a source checkout

The repository SHALL provide an executable Bash installer at the repository root (`install.sh`) that installs the `scryrs` CLI by downloading a published GitHub Release binary, suitable for `curl -fsSL <raw-url>/install.sh | sh` invocation. The installer SHALL NOT require a source checkout, Rust, or Cargo to be present.

#### Scenario: Installer runs from a piped invocation without a checkout

- **WHEN** `install.sh` is executed via `curl ... | sh` on a supported platform with no repository checkout present
- **THEN** the installer downloads a published release binary and installs it
- **AND** it does not invoke `cargo` or require a `Cargo.toml`

#### Scenario: Missing Cargo does not block install

- **GIVEN** `cargo` is not on `PATH`
- **WHEN** `install.sh` runs on a supported platform
- **THEN** the install still completes using the downloaded binary

### Requirement: Installer detects platform and selects the matching release asset

The installer SHALL detect the host operating system and CPU architecture and map them to the matching published release asset target triple. It SHALL support macOS arm64 (`aarch64-apple-darwin`) and Linux x86_64 (`x86_64-unknown-linux-gnu`). On any unsupported OS/arch combination it SHALL exit non-zero with a clear message and SHALL NOT install anything.

#### Scenario: macOS arm64 selects the Darwin asset

- **WHEN** the installer runs on a host reporting `Darwin` and `arm64`/`aarch64`
- **THEN** it selects the `aarch64-apple-darwin` release asset

#### Scenario: Linux x86_64 selects the Linux asset

- **WHEN** the installer runs on a host reporting `Linux` and `x86_64`/`amd64`
- **THEN** it selects the `x86_64-unknown-linux-gnu` release asset

#### Scenario: Unsupported platform fails without mutation

- **WHEN** the installer runs on an OS/arch combination with no published asset
- **THEN** it exits non-zero with a clear unsupported-platform message
- **AND** no install directory or binary is created or modified

### Requirement: Installer verifies the downloaded binary by checksum before installing

The installer SHALL download the asset's published `.sha256` checksum and verify the downloaded binary against it before placing the binary on disk. On checksum mismatch the installer SHALL exit non-zero and SHALL NOT install the binary.

#### Scenario: Checksum matches and install proceeds

- **GIVEN** the downloaded binary matches its published `.sha256`
- **WHEN** the installer verifies the download
- **THEN** verification passes and the binary is installed

#### Scenario: Checksum mismatch aborts the install

- **GIVEN** the downloaded binary does not match its published `.sha256`
- **WHEN** the installer verifies the download
- **THEN** it exits non-zero with a checksum-mismatch message
- **AND** no `scryrs` binary is placed on `PATH`

### Requirement: Installer places the binary on PATH and verifies it runs

The installer SHALL install the verified binary as `scryrs` into a configurable install directory (default `$HOME/.local/bin`, overridable via `--bin-dir <PATH>` or `SCRYRS_INSTALL_DIR`), ensure it is executable, run `scryrs --version` to confirm the install, and print PATH guidance when the install directory is not already on `PATH`.

#### Scenario: Default install location and version check

- **WHEN** the installer runs without overrides
- **THEN** `scryrs` is installed into `$HOME/.local/bin`
- **AND** `<install-dir>/scryrs --version` exits 0

#### Scenario: Install directory override is honored

- **WHEN** the installer runs with `--bin-dir /tmp/scryrs-bin` or `SCRYRS_INSTALL_DIR=/tmp/scryrs-bin`
- **THEN** the binary is installed into that directory

#### Scenario: PATH guidance is shown when the directory is off PATH

- **GIVEN** the chosen install directory is not on the current `PATH`
- **WHEN** the install completes
- **THEN** the installer prints guidance for adding the directory to `PATH`
