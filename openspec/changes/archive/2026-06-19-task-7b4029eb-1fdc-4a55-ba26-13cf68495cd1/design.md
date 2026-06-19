## Context

The v0 CLI contract is frozen and implemented in `crates/scryrs-cli/src/lib.rs`. The argument parser, exit-code policy, JSON output format, and single-command surface are all correct and stable. However, the help text and error messages are bare-bones:

**Current help text:**

```
scryrs - context intelligence for AI-assisted codebases

Usage:
  scryrs hotspots <PATH>

scryrs hotspots emits a versioned JSON summary for the given repository path.
This is a v0 placeholder contract; only this command is defined.
```

**Current error messages** use three different phrasing patterns:

- Unknown command: `"unknown command: {name}"` + `"run \`scryrs --help\`"`
- Missing PATH: `"error: missing required PATH argument"` + `"usage: scryrs hotspots <PATH>"`
- Extra args: `"error: unexpected argument after PATH"` + `"usage: scryrs hotspots <PATH>"`

Only the unknown-command error routes the user toward `--help`. No examples exist in help output. No output format or exit-code documentation is visible to help readers.

**Constraints:**

- No changes to the v0 contract (exit codes, JSON shape, argument parsing, command surface)
- No crate dependencies to add
- Wording must be deterministic and concise for LLM consumption
- All errors go to stderr, all success output to stdout

## Goals / Non-Goals

**Goals:**

- Help text serves as a standalone discovery surface (purpose, usage, arguments, output contract, examples, options, exit codes)
- Error messages use a consistent format and route users toward correct invocation
- Tests validate structural properties of help and error output, minimizing brittleness from text changes
- All changes are confined to `crates/scryrs-cli/src/lib.rs`

**Non-Goals:**

- No changes to the v0 contract, exit-code policy, binary entrypoint, JSON output, or command surface
- No changes to crate dependencies, feature model, or workspace structure
- No `scryrs help` subcommand or other new CLI entry points
- No changes to `README.md`, `cli-v0-contract.md`, `architecture.mdx`, or any docs
- No internationalization, colored output, or terminal formatting
- No man pages, shell completions, or `--help-long` flags

## Decisions

### D1: Help text structure — sectioned reference format

**Decision:** Use clearly delimited uppercase section headings (USAGE, ARGUMENTS, OUTPUT, EXAMPLES, OPTIONS, EXIT CODES) with consistent indentation.

**Rationale:** Sectioned output serves both humans (scanning) and agents (regex anchoring). Uppercase section headers are conventional in Rust CLIs (`cargo`, `rustc`) and are unambiguous as structural markers. The full output fits in ~20 lines, avoiding verbosity while being self-contained.

**Alternatives considered:**

- Prose narrative format — more human-friendly but harder for agents to parse consistently
- Minimal (current state) — fails the "standalone discovery surface" criterion
- `--help` brief + separate detailed output — unnecessary complexity for a single-command CLI

### D2: Help content — include output contract and exit codes

**Decision:** Include the JSON envelope shape (with placeholder values) and a three-row exit-code table in help output.

**Rationale:** If help is the primary discovery surface, an agent reading it must be able to decide:

1. Whether to call the tool (purpose + examples)
2. How to format the invocation (usage + arguments)
3. What response format to expect (output)
4. How to interpret results (exit codes)

Omitting any of these means help is incomplete. The JSON shape is stable (schemaVersion, command, status) and the exit codes are part of the frozen contract.

### D3: Error message format — consistent three-line pattern

**Decision:** Every usage error follows this pattern:

```
<command-target>: <problem statement>
Usage: scryrs hotspots <PATH>
See `scryrs --help`
```

**Rationale:** Three lines = context + remediation + escalation. Every error tells the user what's wrong AND what to do next. The `See \`scryrs --help\`` line is identical across all three errors, making it predictable for agent pattern-matching.

**Specific messages:**

| Condition | Problem | Usage line | Escalation |
|-----------|---------|------------|------------|
| Unknown command | `unknown command: '{name}'` | (omitted — agent likely typed it wrong) | `See \`scryrs --help\`` |
| Missing PATH | `scryrs hotspots: missing required PATH argument` | `Usage: scryrs hotspots <PATH>` | `See \`scryrs --help\`` |
| Extra args | `scryrs hotspots: unexpected argument after PATH` | `Usage: scryrs hotspots <PATH>` | `See \`scryrs --help\`` |

**Alternatives considered:**

- Single-line errors (current unknown-command style) — less informative, no remediation for missing/extra PATH
- Including example in every error — reduces signal-to-noise; one escalation path suffices
- Using `error:` prefix — redundant; the exit code and stderr channel already signal error

### D4: Test strategy — structural assertions over exact strings

**Decision:** Tests assert that help output contains expected section headers, command references, and keyword presence. Error message tests assert structural properties (presence of usage line, escalation line) rather than exact wording.

**Rationale:** Future text refinements should not break tests. Structural assertions (`.contains("USAGE")`, `.contains("EXAMPLES")`, `.contains("See`scryrs --help`")`) validate the contract without freezing copy. The JSON output test retains its exact-string assertion because the JSON schema IS the contract.

**Trade-off:** If the structure itself changes (e.g., section header names), tests must update. This is acceptable — structural changes are intentional and rare.

### D5: No behavior change for `--version` or bare invocation

**Decision:** `--version` and bare invocation remain unchanged in behavior. Bare calls `write_help` (same as `--help`). Version prints `"scryrs <VERSION>"`.

**Rationale:** These paths are already correct per the v0 contract. No UX improvement needed — version output is inherently minimal, and bare invocation already routes to help.

## Risks / Trade-offs

| Risk | Severity | Mitigation |
|------|----------|------------|
| R1: Help text grows stale when JSON schema or exit codes change | Low | These values are v0-contract frozen. Schema version bumps will need a coordinated update across contract docs, JSON output, and help text — the help change will be obvious at that point. |
| R2: Structural test assertions mask formatting regressions (e.g., missing newline between sections) | Low | A missing newline would still pass `.contains()` but would degrade human readability. For v0, this is acceptable risk. If needed, a future task could add snapshot tests. |
| R3: Three-line error pattern adds verbosity to stderr output | Low | ~50 characters per error is negligible. The added clarity for agents (parse-and-retry) justifies the slight verbosity increase. |
| R4: No documentation updates alongside help text changes | None | README, cli-v0-contract.md, and architecture.mdx are explicitly non-goals. This change updates only the runtime help text and error UX. Docs alignment is a separate follow-up. |
