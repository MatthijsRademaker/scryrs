## Why

The `scryrs` codebase already ships a working binary with a real `components` command, tested JSON output, and explicit exit-code behavior. However, the help text advertises 7 stub command names (`trace`, `hotspots`, `propose`, `graph`, `route`, `adapters`) and the `is_known_stub()` dispatcher returns exit 0 with a scaffold message for those names. This creates a de facto broader surface that agents and maintainers can inadvertently depend on, violating the intent that v0 ship only fundamentals. Without an explicit frozen contract, follow-up work may drift into scope-creep before the first public surface is stabilized.

## What Changes

- **Freeze the v0 CLI contract** on the `scryrs` binary with a single public command: `scryrs components`. The `components` command is the only implemented, tested, and README-demonstrated command. It is kept as-is — no rename, no new behavior.
- **Document the contract** in a short design note at `.devagent/docs/docs/cli-v0-contract.md` and add an entry to `.devagent/docs/docs/_nav.json` for discoverability alongside Vision and Architecture docs.
- **Remove the `is_known_stub()` soft-landing dispatch** from `crates/scryrs-cli/src/lib.rs`. Any command name other than `components`, `--help`/`-h`, or `--version`/`-V` must produce the unknown-command flow: stderr guidance message + exit code 2.
- **Narrow `write_help()`** to show only the v0 surface: `scryrs components [--format json]` with global help and version flags. All stub command lines are removed.
- **Codify exit code policy**: 0 for success, help, version; 1 for write/internal CLI failure; 2 for usage error, unknown command, or unsupported invocation.
- **Preserve `--format json` as the machine-readable contract**, including the `schemaVersion` field and `components` array, which are already implemented and tested. The JSON shape is frozen for v0.x.
- **Update `openspec/changes/task-1613d7cf-54a6-44ff-acb3-780736980aa3/`** proposal.md and tasks.md from placeholders to real content.

## Impact

- **Affected specs**: New `cli-v0-contract` capability spec defining the frozen v0 surface.
- **Affected code**: `crates/scryrs-cli/src/lib.rs` (help text narrowing, `is_known_stub()` removal, dispatch simplification). No new functionality is added; existing `components` behavior, `write_components_text()`, `write_components_json()`, and `descriptors()` are unchanged.
- **Affected docs**: `.devagent/docs/docs/cli-v0-contract.md` (new), `.devagent/docs/docs/_nav.json` (add nav entry). README is already scoped to `components` only and requires no changes.
- **Breaking change intent**: Commands that previously returned exit 0 via `is_known_stub()` (e.g., `scryrs trace`, `scryrs hotspots`) will now return exit 2 with a usage error. This is intentional — v0 explicitly means no public contract existed for those names, and the purpose of freezing v0 is to prevent exactly this dependency.
- **No impact on**: Cargo.toml binary naming (already `scryrs`), scryrs-types `SCHEMA_VERSION`, internal feature-gated crate composition, or future command implementations beyond v0 scope.