## Why

The `scryrs` CLI currently ships scaffold code that exposes a real `components` command alongside 8 stub verbs (`trace`, `hotspots`, `report`, `suggest-docs`, `propose`, `graph`, `route`, `adapters`) and prints multi-command help text. The README advertises `scryrs components` with feature-flag examples. This multi-command surface directly violates the v0 contract requirement for exactly one placeholder command with no second real command.

Agent integrators and follow-up feature work will accrete around this throwaway scaffolding unless we freeze a minimal, explicit public surface now. The vision document meanwhile lists future command vocabulary that creates implicit pressure to expand the v0 surface prematurely.

## What Changes

1. **Publish a design note** at `.devagent/docs/docs/cli-v0-contract.md` that freezes:
   - Binary name: `scryrs`
   - Single v0 placeholder command: `scryrs hotspots <PATH>`
   - PATH as a required argument (exit 2 if omitted)
   - Global flags: only `-h`/`--help` and `-V`/`--version` (both exit 0, stdout)
   - Bare `scryrs` (no args): prints help to stdout, exits 0
   - Placeholder command output: versioned JSON envelope on stdout, no human-text fallback
   - Exit codes: 0 (success/help/version), 2 (unknown command / missing PATH / invalid args / unsupported paths), 1 (unexpected runtime failure)
   - All errors and human-facing diagnostics go to stderr
   - Agent-facing contract: when to call, what inputs, what outputs, which paths fail fast
   - Explicit marker that `components`, `trace`, `propose`, `graph`, `route`, `adapters`, `report`, `suggest-docs` are out of scope for v0

2. **Add the design note to docs navigation** in `.devagent/docs/docs/_nav.json` under a new "Technical" section for discoverability.

3. **Update README.md** to remove `components` examples and show only the v0 surface (`--help`, `--version`, or `hotspots <PATH>`).

4. **Update help text** in `crates/scryrs-cli/src/lib.rs` to list only `scryrs hotspots <PATH>` as the available command.

5. **Remove or gate** the `components` command implementation and `is_known_stub` function so code cannot be invoked through the public binary surface.

6. **Update unit tests** in `crates/scryrs-cli/src/lib.rs` to match the new single-command surface.

## Impact

- **Breaking**: The public CLI surface narrows from multi-command scaffold to a single placeholder command. Any automation or docs referencing `scryrs components` or stub verbs will receive exit code 2.
- **Code changes**: `crates/scryrs-cli/src/lib.rs` (argument parsing, help text, removal of `components` and stubs, tests), `README.md`, `.devagent/docs/docs/cli-v0-contract.md` (new), `.devagent/docs/docs/_nav.json`.
- **No engine behavior affected**: scryrs-core, scryrs-graph, scryrs-curator, adapters, and all other workspace crates are unchanged. This is purely a public-surface contract freeze.
- **Vision doc deferred**: vision.md's future command vocabulary is intentionally not updated in this change to avoid scope creep; the design note's out-of-scope declaration covers the conflict.
- **Architecture doc examples**: architecture.mdx uses `scryrs components` in examples. These are not updated in this change per the principle of minimal surface-only changes; the design note acknowledges the divergence and implementation tasks will reconcile in a follow-up.
