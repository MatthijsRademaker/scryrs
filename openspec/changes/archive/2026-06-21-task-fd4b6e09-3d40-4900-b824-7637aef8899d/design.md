## Context

The `scryrs` project has two verification tiers for trace hooks:

1. **Source-hook fixtures** (`scripts/verification/claude-code-e2e.mjs`, `scripts/verification/pi-hook-e2e.mjs`): Load hook artifacts from `hooks/claude-code/scryrs-hook.mjs` and `hooks/pi/index.ts` in the repository source tree. Prove that reference hooks correctly feed `scryrs record --stdin` without changing agent-visible behavior.

2. **Installed-hook fixture** (`scripts/verification/installed-hook-e2e.mjs`): Runs `scryrs init --agent claude-code` and `scryrs init --agent pi` in temporary consumer directories, loads artifacts from consumer install paths (`.claude/hooks/`, `.pi/extensions/pi-trace/`), and exercises them against the real `scryrs` binary. Proves that init output is functional — not just that files were created.

The authoritative Docker-backed verification entrypoint (`scripts/verify-trace-capture`) currently only runs the source-hook fixtures. The installed-hook fixture exists but is orphaned — it cannot be reached through any documented project tooling. The `test-coverage-lane/spec.md` explicitly requires installed-hook end-to-end validation, making this a spec compliance gap.

Additionally, the Claude Code `.claude/settings.json` hook configuration schema is internally inconsistent:
- **`init.rs` next-steps text:** Emits a nested command-block form with `"PreToolUse"` containing `"hooks"` array entries with `"type": "command"` and `"command": "node .claude/hooks/scryrs-hook.mjs"`.
- **`hooks/claude-code/README.md`:** Documents a flat `"hook": "node .claude/hooks/scryrs-hook.mjs"` string form directly inside the `"PreToolUse"` matcher object.

Both forms are valid Claude Code hook registration mechanisms, but they are structurally different. Users following the README will get a different configuration than what the installer guides them to create, creating ambiguity about what constitutes a correct working setup.

The init installer contract (non-mutating for `.claude/settings.json`, refusal on collision) is explicitly specified in `init-installer/spec.md` and must remain unchanged.

## Goals / Non-Goals

### Goals
1. Wire `installed-hook-e2e.mjs` into `scripts/verify-trace-capture` as a third fixture phase, gated behind an `--init-only` flag (matching the existing `--claude-only`/`--pi-only` filter pattern).
2. Reconcile the Claude Code `.claude/settings.json` schema conflict: audit which form is canonical for current Claude Code versions, then update the non-canonical source (either `init.rs` next-steps or `hooks/claude-code/README.md`) to match.
3. Update `scripts/verification/README.md` to document `installed-hook-e2e.mjs` in the fixture tree alongside existing fixtures.
4. Keep the init installer contract unchanged — no auto-creating `.claude/settings.json`, no overwrite behavior.
5. The installed-hook e2e phase explicitly validates the deterministic next-step text accuracy rather than implying zero-touch Claude Code setup.

### Non-Goals
- Do not change the init installer contract (no auto-creating `.claude/settings.json`, no overwrite behavior).
- Do not change hook business logic, TraceEvent schema, or any Rust crate outside verification scripts.
- Do not modify CI configuration, lane assignment, or trigger policies — the verify-trace-capture lane retains its current (NIGHTLY) assignment.
- Do not add new harnesses or change existing harness behavior.
- Do not broaden this into a general test-coverage initiative.

## Decisions

### D1: Add `--init-only` flag to verify-trace-capture

**Decision:** The installed-hook e2e fixture is added as a third phase in `scripts/verify-trace-capture`, invoked after the existing source-hook fixtures when running the full lane, or independently via a new `--init-only` flag.

**Rationale:** Matches the existing `--claude-only`/`--pi-only` filter pattern. Allows developers to run just the init verification without re-running source-hook fixtures. Keeps the full lane additive (all three fixtures run by default) for comprehensive verification. The lead-dev recommended this gating pattern, and all three agents agreed wiring the script into the lane is the correct mechanism.

**Sources:** round:1:agent:swarm-architect, round:1:agent:swarm-lead-dev, round:1:agent:swarm-reviewer

### D2: Reconcile schema by adopting the flat `"hook"` string form as canonical

**Decision:** The canonical Claude Code `.claude/settings.json` hook configuration form is the flat `"hook": "node .claude/hooks/scryrs-hook.mjs"` string form (as documented in `hooks/claude-code/README.md`). The installer's next-steps text in `init.rs` SHALL be updated to emit this form.

**Rationale:** The flat string form is simpler (one line vs. four nested lines), is the documented form in the authoritative hook README, and is the form used by Claude Code documentation and examples. The nested command-block form (`"type": "command"`) is functionally equivalent but introduces unnecessary nesting that adds cognitive load for users manually editing JSON. The README represents the team's canonical documentation; the installer output must match. If auditing reveals that current Claude Code versions require the nested command-block form, this decision SHALL be reversed — the README SHALL be updated to match init.rs instead.

**Sources:** round:1:agent:swarm-architect blocker "The artifact reference-hook documentation and the installer-produced guidance must agree on ONE canonical schema", round:1:agent:swarm-lead-dev "resolve the settings.json schema conflict by auditing which form current Claude Code versions actually accept", round:1:agent:swarm-reviewer blocker "pick one canonical schema form and align both README and installer output to it"

### D3: Keep init installer contract non-mutating

**Decision:** The init installer SHALL NOT be changed to auto-create or modify `.claude/settings.json`. The existing contract (refuse when settings.json exists, instruct user to create it when absent) remains intact.

**Rationale:** The init-installer spec explicitly requires non-mutation. The exploration dossier non-goals and model escalation hints both confirm this boundary. All three refinement agents (architect, lead-dev, reviewer) explicitly recommended keeping the contract unchanged. The installed-hook e2e verification proves next-step text accuracy rather than implying zero-touch setup.

**Sources:** openspec/specs/init-installer/spec.md, decision:1-swarm-architect-recommendation, decision:1-swarm-lead-dev-recommendation, decision:1-swarm-reviewer-recommendation

### D4: Use existing node:22 Docker image for installed-hook e2e

**Decision:** The installed-hook e2e phase in verify-trace-capture SHALL use the same `FIXTURE_NODE_IMAGE` (default `node:22`, Debian glibc) as the existing fixtures.

**Rationale:** The Rust-compiled `scryrs` binary links against glibc. Alpine-based images (musl) cannot run it. `node:22` is already pulled and used by both existing fixtures. The installed-hook script requires `tsx` for Pi TypeScript execution — this is installed via `npm install tsx` inside the container, which adds ~30-60s network-dependent runtime but is acceptable for a NIGHTLY lane.

**Sources:** round:1:agent:swarm-architect risk "The installed-hook e2e script installs tsx via npm inside a temp consumer directory... the Docker-backed lane uses a node:22 image, which has npm available", round:1:agent:swarm-lead-dev risk "Adding installed-hook-e2e.mjs to verify-trace-capture increases runtime... Pi installed-hook test requires npm install tsx via network"

### D5: Update verification README fixture tree

**Decision:** `scripts/verification/README.md` SHALL list `installed-hook-e2e.mjs` in its fixture tree alongside the existing `claude-code-e2e.mjs` and `pi-hook-e2e.mjs` fixtures.

**Rationale:** The current README fixture tree omits `installed-hook-e2e.mjs` despite its existence at the same directory level. This is a documentation gap that all three agents identified. The README is the authoritative index of verification fixtures.

**Sources:** round:1:agent:swarm-architect suggested requirement "scripts/verification/README.md SHALL be updated to document installed-hook-e2e.mjs", round:1:agent:swarm-lead-dev suggested requirement "Documentation for verification SHALL be updated", round:1:agent:swarm-reviewer suggested requirement "scripts/verification/README.md SHALL list installed-hook-e2e.mjs"

## Risks / Trade-offs

| Risk | Severity | Mitigation |
|------|----------|------------|
| R1: Claude Code schema audit may reveal neither form is fully correct for the current Claude Code version, requiring a third form. | Medium | Both the flat `"hook"` string form and the nested command-block form are documented Claude Code hook registration mechanisms. The audit step is a task checkpoint — if neither form works, the task escalates per the model escalation hints in the dossier. |
| R2: Installed-hook e2e adds ~60-90s to the verify-trace-capture lane (npm install tsx, async event waits). | Low | The lane is assigned NIGHTLY with ~41s baseline. Even with the addition, runtime stays well under the 3-minute PR-gate threshold. The `--init-only` flag allows independent invocation for focused debugging. |
| R3: Pi installed-hook e2e requires network access for npm install tsx. | Medium | The lane is NIGHTLY, not PR-gate, so network access is acceptable. If CI moves to air-gapped environments, tsx pre-installation in the Docker image or bundling alternatives should be investigated as a follow-up. The dossier escalation hint explicitly calls this out. |
| R4: Changing init.rs next-steps text changes stdout output, potentially breaking scripts that parse it. | Low | The next-steps text is documented as deterministic but not as a stable API. Existing Rust tests (`init_claude_code_stdout_has_next_steps`) check substrings (`"Next steps"`, `"settings.json"`), not exact byte-level output. These tests will need substring updates but not structural rewrites. |
| R5: If the Pi extension contract requires additional consumer artifacts beyond `index.ts`, the current installed-hook e2e is testing an incomplete init output. | Low | The dossier open question flags this as unresolved. The spec SHALL include a version-gated comment documenting which Pi versions the test is valid for. If Pi adds manifest/config requirements, the init contract and e2e test must be updated together in a follow-up task. |

## Traceability

- **Task:** `fd4b6e09-3d40-4900-b824-7637aef8899d` — Verify Init command
- **Dossier:** `2026-06-21T19:37:13.428Z` — Exploration dossier defining problem, goals, non-goals, assumptions, and open questions
- **Decisions:** `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation` — All round 1 agents converged on tight verification-and-alignment scope: wire installed-hook-e2e into verify-trace-capture, reconcile Claude Code schema conflict, update documentation, keep init contract unchanged
- **Round 1 outputs:** Architect (high confidence, specific requirements for wiring e2e script, schema reconciliation, next-step text verification, README update, Pi version-gating), Lead dev (high confidence, --init-only flag, schema reconciliation, keep init contract unchanged), Reviewer (high confidence, three blockers: schema conflict, orphan script, unmet spec requirement)
- **Evidence:** `scripts/verify-trace-capture` (no reference to installed-hook-e2e.mjs), `scripts/verification/installed-hook-e2e.mjs` (exists and works but is orphaned), `crates/scryrs-cli/src/init.rs` (nested command-block schema in next-steps), `hooks/claude-code/README.md` (flat string schema), `scripts/verification/README.md` (missing installed-hook-e2e from fixture tree), `openspec/specs/init-installer/spec.md` (non-mutating contract), `openspec/specs/test-coverage-lane/spec.md` (requires installed-hook e2e)