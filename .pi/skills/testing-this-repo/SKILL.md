---
name: testing-this-repo
description: "How to build, test, and verify scryrs changes through the Docker-backed scripts — including single-crate runs, snapshot regeneration, and E2E lanes. Use when: running or adding tests, a test fails, you need cargo/clippy/fmt, snapshots need regenerating, or the task mentions scripts/test, scripts/check, scripts/precommit-run, or .docker-fixtures. Do not use for: the TDD method itself (tdd skill covers red/green/refactor; this skill covers HOW to run things here)."
metadata:
  curated: true
  sources:
    - scripts/**
    - .docker-fixtures/**
    - crates/scryrs-cli/tests/**
    - .pre-commit-config.yaml
  last-verified: 2026-07-04
  verified-commit: 8f8ca6cdcee59fb31b4a12f5ab8c48787f2c345c
---

# Testing in This Repo

**Never run cargo, bun, or npm directly on the host** — agent containers have no SDKs; host cargo fails with `command not found`, and even on a dev machine host runs diverge from CI (no `--locked --all-features --all-targets`, no cache volumes). Everything Rust goes through `scripts/lib/docker-verification.sh`'s `run_rust` (image `rust:1.85.0`, repo mounted at `/workspace`).

## Read first

1. `scripts/lib/docker-verification.sh` — the `run_rust` engine every script uses; also the DinD socket selection logic.
2. `scripts/precommit-run` — the authoritative gate: runs `scripts/check` (fmt, frontend check, cargo check, clippy `-D warnings`, docs publish) then `scripts/test` (full workspace).

## Steps

1. Full gate first when the task is near done: run `scripts/precommit-run`. This is authoritative because it chains `scripts/check` then `scripts/test` through Docker-backed wrappers.
   Verify: `scripts/precommit-run` exits 0.
2. Need narrower feedback while iterating: use `scripts/check` for lint/typecheck, `scripts/test` for workspace tests, or `scripts/test --full` when the change touches hook E2E lanes.
   Verify: the specific script you chose exits 0, and you can explain why full precommit is still pending or already rerun.
3. For single-crate or filtered Rust tests, source `scripts/lib/docker-verification.sh` and call `run_rust cargo test -p <crate> <filter> --locked` instead of host cargo.
   Verify: `source scripts/lib/docker-verification.sh && type run_rust` prints the shell function definition.
4. For insta updates, regenerate via Docker only: `source scripts/lib/docker-verification.sh && run_rust env INSTA_UPDATE=always cargo test -p scryrs-cli --locked`, then rerun the same test command without `INSTA_UPDATE` to prove snapshots are stable.
   Verify: `git status --short crates/scryrs-cli/src/snapshots/ crates/scryrs-cli/tests/snapshots/` shows only intended `.snap` changes before the confirming rerun, then no further snapshot diff appears.
5. For hook transport or install-path work, run targeted E2E wrappers like `scripts/verify-trace-capture --pi-only`, `--claude-only`, or `--init-only`; for security advisories use `scripts/security`.
   Verify: the wrapper you chose exits 0.

## Quick command reference

| Need | Command |
|---|---|
| Full verification (authoritative) | `scripts/precommit-run` |
| Lint/typecheck only | `scripts/check` |
| All tests | `scripts/test` (add `--full` to include hook E2E) |
| Single crate / single test | `source scripts/lib/docker-verification.sh && run_rust cargo test -p <crate> <filter> --locked` |
| Regenerate insta snapshots | `source scripts/lib/docker-verification.sh && run_rust env INSTA_UPDATE=always cargo test -p scryrs-cli --locked` — then re-run plain to confirm green |
| Security advisories | `scripts/security` (uses `rust:1.88.0`, not 1.85) |
| Hook capture E2E | `scripts/verify-trace-capture` (`--claude-only` / `--pi-only` / `--init-only`) |
| Skill path lint | `scripts/skill-lint` |

## Test taxonomy

- **Unit**: inline `#[cfg(test)] mod tests` in each crate's source files.
- **Integration**: `crates/<crate>/tests/*.rs` — auto-discovered by Cargo, **no `[[test]]` registration needed**. Exemplar to mirror: `crates/scryrs-cli/tests/doctor_e2e.rs` (spawns `env!("CARGO_BIN_EXE_scryrs")` in a `tempfile::tempdir()`, asserts exit code + JSON).
- **Golden/snapshot (insta)**: help + help-json snapshots in `crates/scryrs-cli/src/snapshots/`, hotspot artifact snapshots in `crates/scryrs-cli/tests/snapshots/`. Byte-for-byte; regenerate deliberately, never hand-edit `.snap` files.
- **E2E verification lanes**: `scripts/verify-*` (trace-capture, live-hotspots, core-artifact-loop, docs-publish, privacy-defaults, install; `scripts/verify-production-suite` composes all).

## Easy-to-miss details

- **clippy denies `Command::spawn/output/status`** (`clippy.toml` disallowed-methods). Integration tests that exec the binary need `#[allow(clippy::disallowed_methods)]` on the calling function (see `doctor_e2e.rs`) or `scripts/check` fails with `-D warnings`.
- **Snapshot regeneration has no wrapper script** — the docs mention bare `cargo insta`; through Docker use the `INSTA_UPDATE=always` form above.
- **`.docker-fixtures/scryrs` is a rebuildable artifact**, overwritten by the E2E lanes (it's a glibc aarch64 binary). Don't assume the committed copy is fresh, and don't debug staleness — the verify scripts rebuild it.
- **glibc vs musl**: E2E fixtures run in `node:22` (Debian). An Alpine Node image cannot exec the fixture binary — don't "optimize" the image choice.
- **Frontend and docs steps escape Docker**: `scripts/check`'s dashboard step and `scripts/verify-docs-publish`'s rspress build call host `bun`/`node`. Without Bun on the host these steps fail — that is an environment limit, not your change; report it rather than weakening the script (see dashboard-ui for the fallback).
- **DinD**: the socket at `/var/run/dind/docker.sock` is used only when `SWARM_DIND_ENABLED=true` AND it exists; otherwise scripts silently fall back to the default socket.

## Self-check before reporting done

- [ ] All verification ran through `scripts/*` — zero host cargo/bun invocations?
- [ ] `scripts/precommit-run` green (or the exact failing step + output included in your report)?
- [ ] New tests fail when the behavior they cover is deliberately broken (no always-green tests)?
- [ ] No `.snap` hand-edits, no `#[ignore]`, no deleted failing tests?
