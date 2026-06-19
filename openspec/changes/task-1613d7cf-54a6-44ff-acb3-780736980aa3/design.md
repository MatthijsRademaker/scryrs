## Context

The `scryrs` project already ships a working binary with a single implemented command (`components`), tested JSON output with `schemaVersion`, and implicit exit-code behavior. However, the help text and dispatch logic still expose 7-8 stub command names with soft-landing exit-0 responses. This creates an over-broad observed surface that contradicts the project's stated intent to ship only fundamentals first. No design note exists yet documenting the v0 contract.

Refinement evidence (swarm-architect, swarm-lead-dev, swarm-reviewer — all round 1, all high confidence) converges on the same direction: freeze on `scryrs components`, remove the `is_known_stub()` dispatch, narrow help, and publish a design note in `.devagent/docs/docs/`. The reviewer flagged the stub dispatch as a blocker that must be resolved; both architect and lead-dev independently proposed the same resolution (remove `is_known_stub()`, fail-fast with exit 2). All open dossier questions are resolved by accepted decisions.

## Goals / Non-Goals

### Goals
- Freeze the first public CLI contract on exactly one command: `scryrs components`
- Document agent-facing invocation rules (when to call, accepted inputs, output shape, stdout/stderr rules)
- Define fail-fast behavior for unknown commands, unsupported flags, and unsupported paths
- Codify exit code policy (0/1/2)
- Remove all stub command names from the public v0 surface (help text and dispatch)
- Preserve the `--format json` machine-readable contract with `schemaVersion`

### Non-Goals
- Implement trace, hotspot, proposal, graph, routing, or adapter behavior
- Expand the public CLI beyond one command plus global help/version flags
- Specify internal engine/storage behavior or long-term command semantics
- Rename `components` — it is already implemented, tested, and README-demonstrated
- Add new global flags, hidden aliases, or backwards-compat shims

## Decisions

### D1: Freeze on `scryrs components` as the single v0 command
**Source**: swarm-architect round 1, swarm-lead-dev round 1, accepted decision `1-swarm-architect-recommendation`
`components` is the only implemented, tested, and README-demonstrated command. The binary name `scryrs` is already explicit in `Cargo.toml`. No rename — renaming before any public consumers buys churn without benefit.

### D2: Remove `is_known_stub()` and enforce fail-fast for unrecognized commands
**Source**: swarm-architect round 1, swarm-lead-dev round 1, swarm-reviewer round 1 (flagged as blocker), accepted decision `1-swarm-architect-recommendation`
The current `is_known_stub()` function recognizes 8 stub command names and returns exit 0 with a scaffold message. This directly violates the acceptance criterion that all unsupported paths must fail fast with a usage-style error. Resolution: remove the `is_known_stub()` function and its dispatch arm so these command names hit the existing unknown-command path (stderr + exit 2).

### D3: Publish the contract note at `.devagent/docs/docs/cli-v0-contract.md`
**Source**: swarm-architect round 1, swarm-lead-dev round 1, accepted decision `1-swarm-architect-recommendation`
This location follows the existing internal docs pattern (alongside Vision and Architecture). The note must also be added to `_nav.json` for discoverability.

### D4: Keep `--format json` as the v0 machine-readable contract
**Source**: swarm-architect round 1, swarm-lead-dev round 1 risk note, accepted decision `1-swarm-architect-recommendation`
JSON output is already implemented, tested, and emits `schemaVersion: 0.1.0` with a `components` array. Removing it would lose the machine-contract requirement. The JSON shape (fields, types, `schemaVersion` semantics) is frozen for v0.x.

### D5: Codify exit code policy as 0/1/2
**Source**: All three round-1 agents, accepted decision `1-swarm-architect-recommendation`
- 0: success (component output, help text, version banner)
- 1: write/internal CLI failure (I/O error writing to stdout/stderr)
- 2: usage error, unknown command, unsupported invocation

## Risks

| Risk | Mitigation |
|------|-----------|
| Stub commands returning exit 0 may already have consumers (CI, scripts). Stripping them to exit 2 is a breaking change. | v0 explicitly means no public contract existed for those names. This breakage is intentional — the purpose of freezing v0 is to prevent the dependency. No mitigation needed beyond documenting the breaking intent. |
| Future `FeatureDescriptor` or `SCHEMA_VERSION` changes could diverge from the frozen JSON output contract. | The JSON output contract is defined in terms of the `components` response shape (field names, types) and versioned via `SCHEMA_VERSION`. Patch-level schema additions within v0.x must be backwards-compatible. |
| README or docs may still imply a broader command set. | README is already scoped to `components` only. Vision doc describes future commands but is explicitly labeled as future direction, not current surface. |

## Traceability

- **Task**: `1613d7cf-54a6-44ff-acb3-780736980aa3` — CLI Foundation 01: Freeze v0 contract and single-command scope v2
- **Dossier**: `2026-06-19T19:59:10.682Z` — problem framing, goals, acceptance criteria, open questions
- **Accepted decisions**: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- **Round outputs**: swarm-architect (round 1), swarm-lead-dev (round 1), swarm-reviewer (round 1)
- **Artifact snapshot**: `proposal-synthesis-input` at `initial` ledger version 1