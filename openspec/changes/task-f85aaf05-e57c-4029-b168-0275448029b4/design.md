## Context

The missing installer is for the `scryrs` binary itself, not for hook configuration. The repository already has `scryrs init --agent <NAME>` for harness hook setup and `scripts/install-hooks` for contributor git hooks, but neither puts `scryrs` on `PATH`. README onboarding still teaches source builds and `cargo run`, and the current release workflow only publishes `scryrs-linux-x86_64`, so a cross-platform downloader would over-promise against the existing distribution pipeline.

## Goals / Non-Goals

### Goals

- Provide a portable macOS/Linux installer script that installs the existing `scryrs` binary into a user-controlled bin directory.
- Build the CLI from the source checkout in release mode with `--locked` using the default feature set, then verify the installed binary with `<install-dir>/scryrs --version`.
- Default to `$HOME/.local/bin`, while allowing `--bin-dir` and `SCRYRS_INSTALL_DIR` overrides.
- Make the installed command immediately usable when the target directory is already on `PATH`, and print exact `PATH` guidance when it is not.
- Keep the CLI binary installer clearly separate from hook installation and shell profile management.
- Add lightweight automated verification for shell syntax and a Linux temp-directory install path using the existing Docker-backed Rust helpers.
- Update README so a first-time user can go from clone to installed `scryrs` without relying on `cargo run` for every invocation.

### Non-Goals

- No Windows installation.
- No Homebrew, apt, npm, cargo-binstall, or other package-manager publishing.
- No `curl | sh` installer that downloads release assets.
- No automatic editing of shell startup files.
- No changes to `scryrs init --agent`, hook behavior, trace schemas, `scryrs record`, or `scryrs hotspots`.
- No release workflow expansion to add macOS artifacts or checksums in this task.

## Decisions

### D1: Source-checkout installer at `scripts/install`

Implement a new executable Bash installer at `scripts/install`. It is a source-checkout installer, not a release-asset downloader, because current repository evidence only supports source builds for macOS and Linux users.

### D2: Platform support is limited to `Linux` and `Darwin`

The installer detects the host OS via `uname` and proceeds only for `Linux` and `Darwin`. Any other value fails non-zero with a clear message and no install mutation.

### D3: Default install target is `$HOME/.local/bin`

The installer targets `$HOME/.local/bin` when no override is supplied. Users can direct installation elsewhere via `--bin-dir <PATH>` or `SCRYRS_INSTALL_DIR`.

### D4: Build the default-feature CLI binary

The installer builds `scryrs-cli` with `cargo build -p scryrs-cli --release --locked` using the default feature set rather than `--all-features`. This preserves the documented CLI command surface while avoiding unnecessary end-user compile cost from optional `llm` and `rspress` features.

### D5: Upgrade behavior is idempotent overwrite

If the chosen install directory already contains `scryrs`, the installer overwrites that target path as normal upgrade behavior. It does not require `--force`, and it does not delete or modify unrelated files in the directory.

### D6: Post-install verification and PATH guidance are mandatory

The installer must run `<install-dir>/scryrs --version` after copying the binary. If the install directory is on `PATH`, `command -v scryrs` in the installer process should resolve to the installed binary. If it is not on `PATH`, the installer prints exact guidance and does not edit shell startup files.

### D7: Binary installation stays separate from hook installation

This change only installs the `scryrs` executable. It must not create or modify `.claude/`, `.pi/`, `.scryrs/`, `scryrs.json`, git hooks, or shell profile files. README must explicitly say that `scryrs init --agent claude-code` and `scryrs init --agent pi` are follow-on hook installers that require `scryrs` on `PATH` first.

### D8: Verification uses existing Docker-backed Rust helpers

Add `scripts/verify-install` to run `bash -n scripts/install` and execute a Linux temp-directory installation inside the existing Rust verification container via `run_rust` from `scripts/lib/docker-verification.sh`.

## Conflict Resolution

- **Feature-set conflict (`--all-features` vs default features):** resolved in favor of the default feature set. The release workflow's `--all-features` build is for artifact publishing, while the accepted implementation guidance for end-user installation is to avoid optional `llm` and `rspress` compile cost.
- **README placement guidance:** refinement agreed on a prominent copy-paste install-from-source path and a clear boundary with `scryrs init --agent`, but did not require a single exact heading order. The specification therefore requires README onboarding to include the install flow and hook-boundary note without inventing stricter placement rules than the accepted evidence supports.

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| macOS execution is not covered by current automated verification. | Medium | Keep `scripts/install` portable to Bash on both `Linux` and `Darwin`, avoid GNU-only behavior, and document that automated execution coverage is Linux-only until CI expands. |
| PATH detection can be confusing when users already have another `scryrs` binary installed. | Medium | Verify the installed binary via `<install-dir>/scryrs --version` and print clear success/PATH messaging based on the chosen install directory. |
| Overwriting an existing `scryrs` binary may replace one installed by another method. | Low | Treat overwrite as expected idempotent upgrade behavior, limit mutation to the target binary path, and avoid touching unrelated files. |

## Traceability

- Task: `f85aaf05-e57c-4029-b168-0275448029b4`
- Dossier: `2026-06-20T21:57:59.942Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Validated round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`