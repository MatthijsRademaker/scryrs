## Why

Task 93754ce0 requests the first runnable native `scryrs` binary with exactly one deterministic placeholder command. The repository already satisfies every acceptance criterion through work delivered in the prior v0-contract freeze change (task-9b98b3fd). The binary target `scryrs` exists at `crates/scryrs-cli`, the entrypoint delegates to `scryrs_cli::run()`, the argument parser accepts only `hotspots <PATH>` plus help/version/bare-invocation, the placeholder emits deterministic versioned JSON, unsupported invocations fail loudly with exit code 2, and no backend wiring is reachable. Eleven unit tests cover all acceptance paths. No code changes are required.

The sole residual drift — stale `scryrs components` examples in `.devagent/docs/docs/architecture.mdx` — was explicitly deferred by the prior change's design and is tracked as a separate follow-up item.

This change documents closure and preserves a traceable audit trail linking this backlog item to its already-landed implementation.

## What Changes

1. **OpenSpec artifacts updated**: `proposal.md`, `design.md`, `tasks.md`, and `specs/cli-foundation-closure/spec.md` now document that the task acceptance criteria are satisfied by current repository state, with citations to the prior change (task-9b98b3fd) implementation evidence.

2. **No code changes**: The `crates/scryrs-cli` crate, `crates/scryrs-types`, `README.md`, `.devagent/docs/docs/cli-v0-contract.md`, and all other repository files are already correct relative to this task's requirements.

3. **Residual drift tracked separately**: Three stale `cargo run -p scryrs-cli -- components` examples in `.devagent/docs/docs/architecture.mdx` (lines 99-101) are acknowledged as known deferred drift from the prior change. This task does not absorb them; they remain a separate docs-alignment work item.

## Impact

- **Backlog hygiene**: Closing this task eliminates a duplicate/overlapping backlog item and prevents wasteful scheduling of work already completed.
- **Traceability**: The closure record in `openspec/changes/task-93754ce0-28cf-4a1b-b755-909aa7018c19/` provides a verifiable audit trail from this task to the prior change's implementation evidence.
- **No operational impact**: No code, binary behavior, contract, or runtime surface changes. All existing behavior, tests, and documentation remain as-is.
- **Docs deferred**: The architecture.mdx stale examples remain a known, explicitly-deferred gap that a follow-up docs-cleanup task should address independently.