## Context

The v0 CLI contract is frozen and implemented through 5 prior Foundation changes: contract freeze, `--help-json`, clap migration, help text improvements, and snapshot tests. The CLI is functionally complete for its v0 scope — one placeholder command (`scryrs hotspots <PATH>`), three global flags (`--help`, `--version`, `--help-json`), three exit codes (0/1/2), and deterministic JSON output.

The gaps this design addresses:

- `README.md` describes workspace structure but offers no path from clone to first command.
- `examples/` is empty (`.gitkeep` only).
- No quickstart exists anywhere in the repo.
- The developer docs (`.devagent/docs/docs/`) serve internal contributors, not first-time users.

The help text already includes an EXAMPLES section — but you have to build and invoke the CLI to see it. The quickstart bridges the gap from "cloned the repo" to "invoked `--help`."

## Goals / Non-Goals

**Goals:**

- A first-time user who clones the repo can read the README and immediately know how to build, run, and understand the CLI.
- All commands in the quickstart are copy-paste runnable from a terminal.
- The quickstart honestly documents current limitations (placeholder only, internals not built).
- Example commands and expected outputs are verified to match the existing `insta` snapshot tests.
- No Rust code, CLI contract, argument parsing, or test behavior changes.

**Non-Goals:**

- No changes to the CLI binary, help text, error messages, exit codes, or JSON output format.
- No additions to the developer-internal docs tree (`.devagent/docs/docs/`).
- No Rust doctests or integration tests for the examples (examples are documentation, not test code).
- No CI enforcement that examples match snapshots (manual verification is sufficient for v0).
- No migration of the help text's EXAMPLES section — it stays as-is, the quickstart is additive.

## Decisions

### D1: Quickstart lives in README.md, not a separate file

**Decision**: Add the Quickstart section directly to the existing `README.md`, between the "Feature split" and "Current status" sections.

**Rationale**:

- The README is the first page a visitor sees on GitHub. A separate `QUICKSTART.md` or `docs/getting-started.md` adds navigation friction — the user must click another link.
- The README is already short (under 2KB). A quickstart section keeps it self-contained.
- The developer docs tree (`.devagent/docs/docs/`) is the wrong audience — it's for internal contributors and swarm agents, not first-time CLI users.
- If the quickstart grows substantially in the future (multi-command surface, engine tutorials), it can be extracted to a dedicated doc. For a single-command v0 placeholder, inline is sufficient.

**Alternatives considered**:

- `QUICKSTART.md` at root: More flexible formatting, but adds friction for the reader. Rejected.
- `.devagent/docs/docs/quickstart.md`: Wrong audience (developer docs). Not added to nav. Rejected.
- `docs/quickstart.md` at root: Doesn't exist yet, adds a new directory at root. Rejected.

### D2: Examples as documented shell commands, not executable scripts

**Decision**: Examples in the quickstart are markdown code blocks showing exact shell commands with expected output. No standalone `.sh` scripts in `examples/`.

**Rationale**:

- The acceptance criteria say "copy-paste runnable" — this is satisfied by well-formatted code blocks that the user can paste directly into their terminal.
- Shell scripts in `examples/` add a maintenance burden (keep script and README in sync) without adding value for a single-command v0 CLI.
- The `examples/` directory is a natural place for future integration examples (multi-step workflows, adapter pipelines, etc.) when engine behavior exists. For v0 placeholder, code blocks in the README are simpler and more discoverable.
- Existing snapshot tests already verify the exact CLI output — the quickstart's expected-output code blocks are implicitly validated.

**Alternatives considered**:

- Shell scripts in `examples/`: Would be correct but adds sync burden. Deferred to post-engine.
- Rust doctests in `lib.rs`: Overengineered for documentation examples. The CLI's `--help` and snapshot tests are the authoritative sources.
- Shell-based integration tests: Would require adding test infrastructure and Docker-backed CI step. Out of scope for a purely documentation change.

### D3: Quickstart covers all surface commands explicitly

**Decision**: The quickstart shows each CLI surface command individually with its exact output: `--help`, `--version`, `--help-json`, `hotspots <PATH>`, and at least one error path.

**Rationale**:

- A first-time user needs to see what each flag does, not just the main command.
- Showing error paths (e.g., missing PATH) demonstrates the CLI's error contract and builds trust — the user sees that failures are handled.
- The task says "inspect help and execute placeholder command" — this requires showing `--help` and `hotspots` separately.
- Each shown output is verifiable against the existing snapshot tests.

### D4: Limitations documented as a separate "Current limitations" section

**Decision**: Add a "Current limitations" subsection within the Quickstart that explicitly states: only one command exists, output is a placeholder JSON envelope, no engine behavior is wired, and speculation about future commands is avoided.

**Rationale**:

- Prevents disappointment from users who expect a working analysis tool.
- The task explicitly requires "Document current limitations honestly: placeholder only, internals not built."
- Separating limitations from the main walkthrough keeps the walkthrough focused on "here's what works" while providing clear expectations.
- No speculative future commands per task requirement — the limitations section uses present-tense descriptions of what does not exist.

## Risks / Trade-offs

| Risk | Mitigation |
|------|-----------|
| R1: Quickstart goes stale when engine behavior lands (help text changes, new commands). | Snapshot tests will fail when output changes, triggering a quickstart update. The quickstart is a single section in README — easy to find and update. |
| R2: Examples in code blocks drift from actual CLI output if someone updates the help text without updating the quickstart. | The quickstart should reference "expected output similar to:" rather than promise byte-exact output. Code blocks show representative output, not copy-paste of the exact snapshot. |
| R3: The quickstart is too short for a dedicated section (single command, nothing to demonstrate). | For v0 this is correct — brevity is a virtue. If the CLI grows, the quickstart naturally expands. A padded quickstart would be worse than a short one. |
| R4: No `examples/` shell scripts means the directory stays empty. | Future engine work will populate it. The `.gitkeep` stays until then. |
