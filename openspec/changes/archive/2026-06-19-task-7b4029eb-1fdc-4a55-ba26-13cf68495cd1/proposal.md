## Why

The v0 CLI contract (`scryrs hotspots <PATH>`) is frozen and working, but its help text and error messages are skeletal — they document *what to type*, not *why*, *what to expect*, or *how to recover*. An agent or human reading `scryrs --help` can discover the syntax but cannot tell what output the command produces, what exit codes mean, or see a concrete invocation example. Error messages use three different phrasing patterns and only one of three paths routes the user toward `--help`. This prevents the CLI from serving as a standalone discovery surface, which is the primary way agents and new users interact with it.

Now is the right time because the contract is stable (no more multi-command surface changes), and every follow-up feature (hotspot scoring, graph commands, curation) will rely on the same help and error UX patterns established here.

## What Changes

1. **Rewrite `write_help` in `crates/scryrs-cli/src/lib.rs`** with sectioned output: purpose, usage, arguments, output contract (JSON schema shape), examples, options, and exit codes. Remove "v0 placeholder contract" implementation-facing language.

2. **Standardize error message format** across all three error paths (unknown command, missing PATH, extra args) to a consistent pattern: `<context>: <problem>\n<usage>\n<next-step>`. Every error includes a remediation hint.

3. **Update unit tests** in `crates/scryrs-cli/src/lib.rs` to match new help text and error messages.

4. **No changes** to the v0 contract, exit-code policy, binary entrypoint, JSON output format, feature model, or any crate outside `scryrs-cli`.

## Capabilities

### New Capabilities

- `cli-discovery-ux`: Help and error message surface that serves as a standalone discovery path for agents and humans. Covers help text content and structure, error message format and routing, and the test suite that validates them.

### Modified Capabilities

- (none — no existing spec has requirement changes; the v0 contract specs remain unchanged)

## Impact

- **Code changes**: `crates/scryrs-cli/src/lib.rs` only — help text, error messages, and tests. No new functions, no new types, no new dependencies.
- **No contract change**: Exit codes, JSON output format, argument parsing behavior, and command surface are all unchanged.
- **No engine crate changes**: `scryrs-core`, `scryrs-graph`, `scryrs-types`, and all other workspace crates untouched.
- **Test updates**: ~5 unit tests that assert exact error or help strings will need updating. Structural assertions (`.contains()`) minimize future brittleness.
