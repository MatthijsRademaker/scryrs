## 1. Add the CLI binary installer

- [x] 1.1 Create an executable Bash script at `scripts/install`.
- [x] 1.2 Detect the host OS via `uname`; allow only `Linux` and `Darwin`, and fail non-zero without mutation on unsupported values.
- [x] 1.3 Resolve the install directory from `$HOME/.local/bin` by default, with overrides from `SCRYRS_INSTALL_DIR` or `--bin-dir <PATH>`.
- [x] 1.4 Build `scryrs-cli` in release mode with `cargo build -p scryrs-cli --release --locked` using the default feature set.
- [x] 1.5 Copy `target/release/scryrs` into the chosen install directory as `scryrs`, ensure it is executable, overwrite an existing target binary as normal upgrade behavior, and leave unrelated files untouched.
- [x] 1.6 Verify the install by running `<install-dir>/scryrs --version` and print exact `PATH` guidance when the chosen directory is not already on `PATH`.
- [x] 1.7 Ensure the installer does not create or modify `.claude/`, `.pi/`, `.scryrs/`, `scryrs.json`, git hooks, or shell profile files.

## 2. Add automated installer verification

- [x] 2.1 Create `scripts/verify-install`.
- [x] 2.2 Run `bash -n scripts/install` as part of installer verification.
- [x] 2.3 Use `scripts/lib/docker-verification.sh` and `run_rust` to execute a Linux temp-directory install inside the existing Rust verification container.
- [x] 2.4 Assert that the verification flow confirms `<temp-install-dir>/scryrs --version` succeeds.

## 3. Update README onboarding

- [x] 3.1 Add copy-paste macOS/Linux install-from-source instructions for the new installer.
- [x] 3.2 Clarify that `scripts/install` installs the CLI binary, while `scryrs init --agent claude-code` and `scryrs init --agent pi` install hooks only after `scryrs` is on `PATH`.

## 4. Validate scope boundaries

- [x] 4.1 Confirm the change does not add Windows support, package-manager distribution, release-asset downloading, or shell profile automation.
- [x] 4.2 Confirm the change does not alter hook behavior, trace schemas, `scryrs record`, or `scryrs hotspots`.
