## Context

The `scryrs` repository already ships a scaffolded CLI (`crates/scryrs-cli`) with a real `components` command, 8 stub verbs, and multi-command help text. The README advertises `scryrs components` with feature-flag variants. The task requires freezing a v0 public CLI contract narrowed to exactly one placeholder command, explicit global flags, exit-code policy, and agent-facing output contract before any more commands or engine behavior land.

The existing architecture docs describe the CLI as a "scaffold-level, feature-gated command surface" — confirming that the external surface can be contracted without touching internal engine crates.

The vision document (`vision.md`) lists future command vocabulary (`scryrs trace`, `scryrs hotspots`, `scryrs propose`, `scryrs graph`, `scryrs route`, `scryrs adapters`) that creates implicit pressure to expand the v0 surface. The contract note must explicitly mark all vision verbs except `hotspots` as out of scope for v0.

## Goals

1. Freeze `scryrs` as the binary name and `scryrs hotspots <PATH>` as the sole v0 placeholder command.
2. Define PATH as a required argument (not optional-with-CWD-default) for unambiguous agent behavior.
3. Define exactly two global flags (`-h`/`--help`, `-V`/`--version`) with human-readable stdout and exit 0.
4. Define a versioned JSON stdout contract for the placeholder command (no human-text fallback).
5. Define exit-code policy: 0 (success/help/version), 2 (unknown commands, missing PATH, invalid args, unsupported paths), 1 (unexpected runtime failure).
6. Define bare `scryrs` behavior: help to stdout, exit 0.
7. Define agent-facing contract: when an agent should call `scryrs hotspots`, what input it expects, what output it returns, and which paths must fail fast.
8. Remove or gate `components`, `is_known_stub`, and all stub-command code from the public binary surface.
9. Update README, help text, and docs navigation to reflect the single-command v0 surface.
10. Publish a short CLI contract design note at `.devagent/docs/docs/cli-v0-contract.md` discoverable via `_nav.json`.

## Non-Goals

- No implementation of trace collection, hotspot analysis, or any internal engine behavior.
- No design or preservation of the eventual multi-command suite beyond the single v0 placeholder.
- No backwards compatibility for `components` or any current scaffold verb.
- No update to `vision.md`'s future command vocabulary (the design note's out-of-scope declaration covers the conflict).
- No update to `architecture.mdx`'s `scryrs components` examples in this change.
- No JSON schema definition beyond the minimal v0 placeholder envelope.

## Decisions

### D1: Placeholder command — `scryrs hotspots`
**Decision**: `scryrs hotspots` is the single v0 placeholder command.
**Rationale**: Aligns with the product tagline ("Find context hotspots"), the standalone detector narrative in vision.md, and architecture.mdx's description of scryrs-core as "standalone trace ingestion, event model, and hotspot detector." Does not require trace-collection infrastructure to be implemented first.
**Sources**: swarm-architect (round 1), swarm-lead-dev (round 1), swarm-reviewer (round 1), vision.md, architecture.mdx.

### D2: PATH argument — required (no CWD default)
**Decision**: `scryrs hotspots <PATH>` requires exactly one PATH argument. Zero arguments is a usage error (exit 2).
**Rationale**: An explicit required argument prevents ambiguous agent behavior. CWD default is a convenience for humans but a silent misbehavior vector for agents. An agent that omits PATH intentionally will get exit 2, not a result from the wrong directory.
**Conflict resolved**: swarm-architect recommended CWD default; swarm-lead-dev recommended required PATH. Resolved in favor of required PATH because agent safety (preventing silent wrong-directory results) outweighs human convenience (typing `scryrs hotspots .` is trivial).
**Sources**: swarm-lead-dev (round 1), swarm-reviewer blocker-3 (round 1).

### D3: Output contract — versioned JSON on stdout
**Decision**: The placeholder command always emits a versioned JSON object to stdout. No human-readable text fallback for the placeholder command. The JSON envelope includes a `schemaVersion` field matching `scryrs-types::SCHEMA_VERSION`.
**Rationale**: Agent integrators need a stable machine-readable contract. Human-readable prose output for the command would change between versions and create parsing fragility. Help/version remain the human-readable paths.
**Sources**: swarm-architect (round 1), swarm-lead-dev (round 1), swarm-reviewer blocker-2 (round 1).

### D4: Bare `scryrs` — help + exit 0
**Decision**: Invoking `scryrs` with no arguments prints help text to stdout and exits 0.
**Rationale**: Matches current scaffold behavior and standard CLI conventions (e.g., `cargo`, `git`). Changing to a usage error adds no agent-facing value — agents should use `--help` or a specific command, and bare invocation as a usage error would surprise humans with CLI muscle memory.
**Sources**: swarm-architect (round 1), swarm-lead-dev (round 1), swarm-reviewer blocker-1 (round 1).

### D5: Exit-code policy
**Decision**:
- 0: successful command execution, help display, version display
- 2: unknown commands, missing PATH, invalid arguments, unsupported paths (usage errors)
- 1: unexpected runtime failures (I/O errors, internal panics)
**Rationale**: Three-tier scheme matches POSIX conventions and current code (exit code 2 already used for unknown commands). Separates usage errors from runtime failures so agent integrators can distinguish contract violations from transient failures.
**Sources**: swarm-architect (round 1), swarm-lead-dev (round 1), current lib.rs behavior.

### D6: Design note location — `.devagent/docs/docs/cli-v0-contract.md`
**Decision**: The CLI contract design note lives at `.devagent/docs/docs/cli-v0-contract.md` with a navigation entry in `_nav.json` under a new "Technical" section.
**Rationale**: The internal docs tree is the established place for architecture and vision notes. Adding a contract note here satisfies the "design note exists in repo" acceptance criterion and keeps it discoverable alongside existing design documentation.
**Sources**: swarm-architect (round 1), swarm-lead-dev (round 1), _nav.json current structure.

### D7: `components` and stub removal
**Decision**: The `components` command implementation (`write_components_text`, `write_components_json`, `descriptors` function) and `is_known_stub` function are removed from the public binary surface. If preservation is needed for internal testing, they may be gated behind a non-default feature flag (`_dev` or `unstable`).
**Rationale**: AC-4 forbids a second real command. `components` is a real (non-stub) command with JSON output and tests. Leaving it reachable in the binary violates the v0 contract. Stub commands printing friendly placeholders (exit 0) would mislead agent integrators into building against an unsupported surface.
**Sources**: swarm-lead-dev (round 1), swarm-architect RISK-1 (round 1).

## Risks

| Risk | Mitigation |
|------|-----------|
| R1: `components` and stub code exist after contract note is published. Implementation tasks must remove or gate them so they fail fast (exit 2). | Explicit removal task in tasks.md. |
| R2: README diverges from v0 contract if not updated alongside help text. | README update task tied to help-text update task. |
| R3: vision.md lists future commands that conflict with single-command v0 surface. | Design note explicitly marks all vision verbs except `hotspots` as out of scope for v0. |
| R4: architecture.mdx uses `scryrs components` examples. Readers following those examples will get exit 2. | Design note acknowledges the gap. Architecture doc update deferred to avoid scope creep; this is a known, documented discrepancy. |
| R5: Committing to `hotspots` as the placeholder may conflict if product vocabulary later shifts toward `trace` or `record`. | Design note labels this as a v0 placeholder contract, not a permanent naming commitment. Provision for deprecation/rename in v1. |
| R6: Existing unit tests test `components` and `unknown_command` behavior. | Tests must be updated to match the new single-command surface. |

## Traceability

- Task: `9b98b3fd-93f6-4b08-b4b5-cb30e5f9d486`
- Exploration dossier: `2026-06-19T20:17:03.384Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round 1 agent outputs: swarm-architect, swarm-lead-dev, swarm-reviewer
- Repository sources: `crates/scryrs-cli/src/lib.rs`, `README.md`, `.devagent/docs/docs/vision.md`, `.devagent/docs/docs/architecture.mdx`, `.devagent/docs/docs/_nav.json`, `crates/scryrs-cli/Cargo.toml`, `crates/scryrs-types/src/lib.rs`