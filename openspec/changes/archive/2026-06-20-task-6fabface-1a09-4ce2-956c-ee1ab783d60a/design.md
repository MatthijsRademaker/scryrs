## Context

The `scryrs` CLI currently serves two commands (`hotspots` and `record`) with a pre-clap command whitelist, clap builder API dispatch, and `--help-json` machine-readable surface at `SURFACE_VERSION 0.2.0`. Two reference hook implementations exist under `hooks/claude-code/` and `hooks/pi/`, each with source artifacts and READMEs documenting manual installation steps. The trace-hook-contract documentation explicitly states "Hook installation is currently a manual process pending `scryrs init --agent <name>`." The roadmap requires this installer as a Phase 1 deliverable before hotspots, graph generation, route manifests, or LLM features.

The installer must bridge the gap between "reference hook sources exist" and "harness integrators can deterministically install them." It must produce a self-contained binary (no runtime dependency on a `hooks/` source tree), refuse unsafe operations (self-install into the scryrs source checkout, unmanaged config collision), and emit deterministic output suitable for scripted callers.

## Goals / Non-Goals

### Goals

- A harness integrator can run `scryrs init --agent claude-code` or `scryrs init --agent pi` and receive a working reference hook installation in a project-local consumer directory.
- The binary is self-contained — hook source file contents are embedded at compile time via `include_str!()`, so distributed binaries work without a `hooks/` source tree.
- `scryrs init --agent <unknown>` exits with a deterministic non-zero usage error and lists supported harnesses in stable order.
- Successful installations print exact next steps per harness (reload instructions, PATH guidance, follow-up config) with deterministic wording.
- Pre-existing target files trigger loud refusal (exit 2) with remediation instructions; no silent partial mutation.
- Self-install into the scryrs source checkout is detected and refused with a clear error.
- All CLI surfaces (`--help`, `--help-json`, root command dispatch) reflect `init` without regressing `record`/`hotspots`.
- All existing tests pass unchanged; new tests cover init behavior exhaustively.

### Non-Goals

- No new harnesses beyond `claude-code` and `pi` — only the two existing reference hook implementations are supported.
- No structured JSON merge for `.claude/settings.json` — loud refusal with remediation only. A `--force` escape hatch is deferred.
- No `--target-dir` flag in v1 — target is always project-local relative to CWD.
- No creation of `scryrs.json` manifest — the installer neither creates nor depends on it.
- No modification of `.gitignore` patterns for consumer-side installed files.
- No changes to hook business logic, TraceEvent schema, `scryrs record`, or any Rust crate outside `scryrs-cli`.
- No `SessionEnd` emission or session lifecycle changes for hooks.
- No per-command `--help-json` for `init` — the global surface document covers it.

## Decisions

### D1: Asset embedding via `include_str!()` (not runtime filesystem reads)

**Decision:** Hook source file contents (`hooks/claude-code/scryrs-hook.mjs`, `hooks/pi/index.ts`) are embedded in the binary at compile time using `include_str!()` with relative paths from `crates/scryrs-cli/src/`.
**Rationale:** Runtime filesystem reads from `hooks/` would break when the binary is installed via `cargo install` or distributed as a standalone artifact (no source tree present). `include_str!()` produces a self-contained binary and is the zero-dependency Rust pattern for compile-time asset embedding. The `hooks/` directory remains the canonical source of truth; the build-time path relationship is documented in a comment and verifiable via a build-time integration test.
**Sources:** Architect round 1 blocker 3; Lead dev round 1 recommendation; Reviewer blocker B1.

### D2: Separate `init.rs` module with typed harness registry

**Decision:** The `init` subcommand implementation lives in a new `crates/scryrs-cli/src/init.rs` module containing a public `execute_init(out, err, agent_name) -> i32` function and a private typed `HarnessRegistry` with entries for `claude-code` and `pi`. Each entry declares: agent name, source asset (`include_str!` path), target relative directory, target filename, and deterministic next-step text.
**Rationale:** The existing `lib.rs` is ~1295 lines. Inlining installer logic would worsen maintainability. The task's technical notes explicitly require "installer logic separate from hook business logic." A typed registry makes harness additions mechanical (add one struct literal) and keeps test coverage surface well-bounded.
**Sources:** Architect round 1 recommendation; Lead dev round 1 recommendation.

### D3: Pre-clap whitelist addition for `init`

**Decision:** Add `"init"` to the pre-clap known-command condition in `lib.rs` alongside `"hotspots"` and `"record"`. Define `init` as a clap subcommand with a required `--agent <NAME>` argument. Dispatch successful matches to `init::execute_init`.
**Rationale:** The pre-clap guard at lines ~81-90 explicitly whitelists commands before clap processes arguments. Without adding `"init"`, `scryrs init --agent claude-code` would exit 2 as an unknown command. The clap subcommand handles `--agent` validation (missing, empty) with standard usage error formatting consistent with existing `record` and `hotspots` error patterns.
**Sources:** Architect round 1 blocker 1; Lead dev round 1 recommendation.

### D4: Project-local Pi target (`.pi/extensions/pi-trace/`)

**Decision:** The Pi installer writes to `.pi/extensions/pi-trace/index.ts` relative to CWD. The `hooks/pi/README.md` documents both project-local (`.pi/extensions/pi-trace/`) and user-global (`~/.pi/agent/extensions/pi-trace/`) targets; v1 defaults to project-local for consistency with Claude Code's `.claude/hooks/` pattern and to enable self-install boundary detection via CWD ancestry checks.
**Rationale:** Project-local matches Claude Code's `.claude/` pattern, avoids `$HOME` resolution edge cases, and keeps the installation scope bounded to a single project. Users who want user-global installations can symlink. A `--scope` flag could be added later without breaking v1 behavior.
**Sources:** Dossier open question; Lead dev risk; Reviewer question Q2.

### D5: Claude Code settings.json collision — loud refusal only

**Decision:** If `.claude/settings.json` already exists in the target directory, the installer exits 2 with a deterministic error message listing the exact JSON block the user must insert manually and the setting's purpose. No structured merge or partial rewrite is attempted.
**Rationale:** The model escalation hint in the exploration dossier explicitly warns: "Escalate if the team tries to auto-merge arbitrary existing `.claude/settings.json`; deterministic failure is safer than clever JSON surgery." The file may contain user-maintained settings unrelated to hooks. Attempting JSON merge risks silent corruption. Refusal with exact insertion instructions is safer, deterministic, and testable.
**Sources:** Architect round 1 recommendation; Reviewer blocker B3.

### D6: Self-install boundary detection via dual markers

**Decision:** The installer detects the scryrs source checkout by checking for the presence of both a `Cargo.toml` referencing `scryrs-cli` in `[workspace.members]` AND a `hooks/claude-code/` directory in the resolved target path ancestry. If both markers are found, the installer exits 2 with an error explaining the reference-only boundary.
**Rationale:** A single marker (Cargo.toml with scryrs-cli) could false-positive on a user project that includes scryrs as a workspace member. The dual-marker heuristic (Cargo.toml workspace marker + hooks/claude-code/ directory) makes false positives extremely unlikely while catching all genuine self-install attempts. This is the most impactful architectural constraint — getting it wrong either enables accidental repo contamination or frustrates legitimate use.
**Sources:** Architect round 1 blocker 2 and risk 1; Reviewer blocker B2.

### D7: SURFACE_VERSION bump to 0.3.0

**Decision:** `SURFACE_VERSION` increments from `"0.2.0"` to `"0.3.0"` as a minor bump in accordance with the `cli-v0-contract.md` versioning policy for additive changes (new command added).
**Rationale:** Adding a new command to the CLI surface is an additive change that doesn't break the existing surface document schema. The `cli-v0-contract.md` versioning policy specifies minor bumps for additive changes.
**Sources:** Lead dev round 1 recommendation; Reviewer risk R1.

### D8: No `scryrs.json` creation

**Decision:** The installer does not create, read, or depend on the `scryrs.json` manifest. The hook contract documents it as a provisional v0.1 shape; no checked-in manifest exists.
**Rationale:** The trace-hook-contract marks `scryrs.json` as provisional and subject to change. Creating it prematurely would commit to an unstable schema. The dossier exploration open question asks "Should the installer create `scryrs.json`, or only mention it?" and the architect decision explicitly says "The installer must NOT create the provisional `scryrs.json` manifest."
**Sources:** Architect round 1 recommendation; Dossier open question.

### D9: Exit code contract for init

**Decision:** `init` follows the existing 0/1/2 exit code contract: 0 for successful install, 1 for I/O errors (write failure), 2 for usage errors (unsupported harness, collision, self-install refusal, missing `--agent`).
**Rationale:** Consistent with the existing `record` and `hotspots` exit code semantics documented in `cli-v0-contract.md`. Scripted callers can rely on exit code 2 for all non-retryable usage errors.
**Sources:** Task acceptance criteria ("deterministic for scripted callers"); existing exit code contract.

## Risks / Trade-offs

| Risk | Severity | Mitigation |
|------|----------|------------|
| R1: `include_str!` paths are fragile if `crates/scryrs-cli/` is moved. | Low | The path relationship (`../../../hooks/claude-code/scryrs-hook.mjs`) is documented in a comment in `init.rs`. A build-time integration test asserts both `include_str!` calls compile and return non-empty strings. If the crate is moved, the compiler fails at build time — no silent runtime breakage. |
| R2: Self-install boundary dual-marker heuristic could false-positive. | Low | Requiring both `Cargo.toml` with `scryrs-cli` in workspace members AND `hooks/claude-code/` directory means a user project must include scryrs as a workspace member AND clone the hooks directory structure, which is extremely unlikely outside the scryrs source checkout. |
| R3: Claude Code settings.json refusal is user-hostile for existing manual installations. | Medium | The error output includes the exact JSON block to paste and clear instructions. A `--force` flag for overwriting managed blocks is deferred to a follow-up task. Documenting this in the help text and error output reduces friction. |
| R4: Pi install writes to `.pi/extensions/pi-trace/` which requires a `pi.json` manifest for Pi auto-discovery that `hooks/pi/` does not include. | Medium | The installer copies only `index.ts` (the reference hook source). The next-step text instructs the user to run `/reload` in Pi, which triggers extension auto-discovery from directory structure. If Pi requires a `pi.json`, the next-step text should mention this. The actual `pi.json` creation is deferred as it's a Pi harness concern. |
| R5: Snapshot tests (`insta::assert_snapshot!`) will break when help text changes. | Low | Snapshot updates are mechanical — run `cargo test` with `INSTA_UPDATE=1` or `cargo insta review` to accept the new snapshots. |
| R6: `previously_stubbed_commands_exit_2` test does not list `init` — if `init` is accidentally excluded from the pre-clap whitelist, it would fall through to the stubbed-commands test and exit 2, creating a false-positive test pass for a broken implementation. | Low | The test is updated by removing `init` from the assert list but adding an explicit test that `init` (without `--agent`) exits 2 via clap usage error, not via the unknown-command path. |

## Traceability

- **Task:** `6fabface-1a09-4ce2-956c-ee1ab783d60a` — Trace Foundation 06: Add `scryrs init --agent` installer
- **Dossier:** `2026-06-20T17:41:02.717Z` — Exploration dossier defining problem, goals, non-goals, assumptions, and open questions
- **Decisions:** `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation` — All round 1 agents approved the proposal sketch with specific architectural refinements (embed, separate module, pre-clap whitelist, collision refusal, boundary detection)
- **Round 1 outputs:** Architect (high confidence, specific requirements for init.rs, include_str!, whitelist, settings.json refusal, self-install detection), Lead dev (high confidence, SURFACE_VERSION bump, project-local Pi target, snapshot updates), Reviewer (medium confidence, three blockers now resolved by architect/lead-dev decisions)
- **Evidence:** `crates/scryrs-cli/src/lib.rs` (pre-clap whitelist, SURFACE_VERSION, cli_surface_doc, write_help), `hooks/claude-code/scryrs-hook.mjs`, `hooks/pi/index.ts`, `hooks/claude-code/README.md`, `hooks/pi/README.md`, `.devagent/docs/docs/trace-hook-contract.md`, `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/roadmap.mdx`, `README.md`