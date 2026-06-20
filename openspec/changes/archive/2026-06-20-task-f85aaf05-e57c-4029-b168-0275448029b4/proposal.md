## Why

The repository still lacks a supported way to put the `scryrs` CLI on a user's `PATH`. Existing installer-like flows solve different problems: `scripts/install-hooks` installs contributor git hooks, and `scryrs init --agent <NAME>` installs consumer hook files only after `scryrs` is already available on `PATH`. The README still centers source builds and `cargo run`, while the release workflow only publishes a Linux artifact, so a remote macOS/Linux downloader is not a viable v1.

## What Changes

1. **Add a source-checkout CLI installer** at `scripts/install` for macOS and Linux. The script builds `scryrs-cli` from the repository with `cargo build -p scryrs-cli --release --locked` using the default feature set, installs the `scryrs` binary into a configurable bin directory, and verifies the installed binary with `<install-dir>/scryrs --version`.
2. **Make installation deterministic and user-controlled.** The installer defaults to `$HOME/.local/bin`, allows `--bin-dir` and `SCRYRS_INSTALL_DIR` overrides, overwrites an existing target `scryrs` binary as idempotent upgrade behavior, and prints exact `PATH` guidance when the chosen directory is not already on `PATH`.
3. **Keep binary installation separate from hook installation.** The installer manages only the CLI binary, does not edit shell startup files, and does not create or modify `.claude/`, `.pi/`, `.scryrs/`, `scryrs.json`, or git hooks. README will explicitly distinguish `scripts/install` from `scryrs init --agent <NAME>`.
4. **Add lightweight automated verification** via `scripts/verify-install`, covering `bash -n scripts/install` and a Linux temp-directory install executed through the existing Docker-backed `run_rust` helper in `scripts/lib/docker-verification.sh`.
5. **Update README onboarding** with copy-paste install-from-source instructions for macOS/Linux and a clear note that hook installation comes after the CLI binary is installed and reachable on `PATH`.

## Impact

- **New files:** `scripts/install`, `scripts/verify-install`.
- **Updated docs:** `README.md`.
- **No release/distribution expansion:** no Homebrew, apt, npm, cargo-binstall, remote release downloader, or release-matrix expansion.
- **No platform expansion beyond scope:** no Windows installer.
- **No hook/runtime behavior changes:** no changes to `scryrs init --agent`, trace schemas, `scryrs record`, `scryrs hotspots`, or shell profile automation.
- **Verification scope remains intentionally narrow:** automated execution coverage is Linux-only; macOS support relies on portable Bash constructs and documented/manual validation until CI expands.