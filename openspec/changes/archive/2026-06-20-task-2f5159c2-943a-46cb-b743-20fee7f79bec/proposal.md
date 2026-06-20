## Why

The repository appears to have reached the roadmap's Phase 1 deterministic proxy-capture boundary in code, but it cannot honestly be declared complete while authoritative docs and metadata still describe shipped behavior as forthcoming or document stale contracts. A few small trust-boundary gaps also remain: record ingestion accepts semantically incoherent `TraceEvent` lines, the Claude settings collision message claims work that has not happened, and the Pi hook does not report resolved non-zero `scryrs record` exits.

## What Changes

1. Publish a Phase 1 closure matrix that maps each roadmap deliverable to the current code, manifest, hook, installer, and verification artifacts, including accepted limitations.
2. Sync the Phase 1-facing surfaces with the shipped v0 boundary by updating `.devagent/docs/docs/roadmap.mdx`, `.devagent/docs/docs/trace-hook-contract.md`, `README.md`, and `scryrs.json` so they describe the existing `record` command, append-only persistence, checked-in hooks, `scryrs init --agent`, current help/help-json surface, and accepted lifecycle limitations.
3. Harden ingestion by rejecting semantically invalid `TraceEvent` lines when `schema_version` differs from `SCHEMA_VERSION` or `event_type` disagrees with `payload.type`, while preserving `record`'s partial-accept/continue behavior and deterministic rejection diagnostics.
4. Close the remaining trust-boundary bugs by correcting the misleading `.claude/settings.json` collision message in `crates/scryrs-cli/src/init.rs` and making the Pi hook log fail-open diagnostics for resolved non-zero `pi.exec(...).code` results as well as thrown errors.
5. Re-run Docker-backed validation with `scripts/check`, `scripts/test`, and `scripts/verify-trace-capture`, extending verification coverage for the Pi non-zero exit path and any new record rejection cases.

## Impact

- Touches project docs/metadata, `scryrs record` ingestion, `scryrs init`, the Pi reference hook, and verification fixtures/scripts.
- Preserves the current Phase 1 product boundary: no real hotspot materialization, no graph/proposal/adapter/runtime/LLM work, and no new harnesses.
- Keeps accepted harness limitations explicit: Claude Code remains PreToolUse-only, and Pi still captures `SessionStart` but not `SessionEnd`.