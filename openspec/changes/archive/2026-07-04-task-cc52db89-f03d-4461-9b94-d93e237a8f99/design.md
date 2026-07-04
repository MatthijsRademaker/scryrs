## Context

The `scryrs` CLI currently serves a single v0 placeholder command (`hotspots <PATH>`) with global flags (`-h`/`--help`, `-V`/`--version`) and bare-invocation help. The argument parser in `crates/scryrs-cli/src/lib.rs` is hand-written (no clap/structopt dependency). The help text is prose — informative for humans but ambiguous for programmatic callers.

An LLM agent cannot deterministically discover the CLI surface without either:

- Parsing prose help text (`--help`) and hoping its reading comprehension survives version changes, or
- Calling commands speculatively and inferring the surface from exit codes and error messages.

This design adds a machine-readable surface description as a `--help-json` flag that outputs a versioned JSON document describing the complete CLI contract — commands, arguments, flags, output shapes, and exit codes. The approach is additive: no existing behavior changes, no new dependencies, no changes outside `scryrs-cli`.

## Goals / Non-Goals

**Goals:**

- An LLM agent can run `scryrs --help-json` and receive a structured JSON document describing all available commands, their arguments, flags, output contracts, and exit code semantics.
- The surface document is versioned independently (a `surfaceVersion` field) so agents can detect format changes.
- The output includes per-command output contract metadata (MIME type, field-level schema of the output envelope) so agents know what to parse before invoking.
- The flag follows the same contract as `--help` and `--version`: emitted to stdout, exit 0.
- All existing behavior and all existing tests remain passing.

**Non-Goals:**

- No changes to the command output format, exit codes, error messages, argument parsing, or any existing contract.
- No JSON Schema ($schema) output — the surface document is a lightweight JSON object, not a formal JSON Schema document. A formal schema surface could be added later if needed.
- No shell completion generation (bash/zsh/fish). This is not a replacement for shell completions; it's an agent-facing surface.
- No MCP tool definition generation. The surface document could feed MCP tool definitions in a later integration, but that is out of scope here.
- No changes to `scryrs-types`, `scryrs-core`, or any crate outside `scryrs-cli`.

## Decisions

### D1: `--help-json` as the flag name

**Decision**: Use `--help-json` as the flag name (and `-hj` as a short form).
**Rationale**: Follows the `--help` naming convention. The `-json` suffix makes the format explicit. The flag sits alongside `--help` (prose) and `--version` (string) as a third introspection mechanism. Alternatives considered: `--cli-json` (less conventional, less discoverable), `--describe` (ambiguous — describes what?), `--surface` (too abstract, could conflict with future subcommands).
**Sources**: Existing `--help` and `--version` convention in the CLI.

### D2: Independent surface version

**Decision**: The surface document includes a `surfaceVersion` field (e.g., `"0.1.0"`) that is **separate** from the command output's `schemaVersion`.
**Rationale**: The surface description format evolves independently of the command output format. A new command might be added to the CLI without changing the output envelope of `hotspots`. Using a single version for both would couple unrelated concerns. Using semver: major bumps for breaking surface doc format changes, minor for additive changes (new commands/flags), patch for clarifications.
**Sources**: The existing `SCHEMA_VERSION` in `scryrs-types` is explicitly tied to output envelope format, not CLI surface description.

### D3: Lightweight inline field schema (not full JSON Schema)

**Decision**: Per-command output contract metadata uses a lightweight `fields` array describing each output field (`name`, `type`, `description`, `optional`) rather than a full JSON Schema document.
**Rationale**: The current output envelope has exactly 3 fields. A full JSON Schema adds a heavyweight dependency (`schemars` or similar) and produces 5× the markup for the same information. A simple inline array is sufficient for the v0 three-field envelope and trivially extensible. If the output format grows significantly, migrating to JSON Schema can be done without breaking `surfaceVersion` — the field schema is advisory, not authoritative.
**Sources**: Current output envelope shape (`schemaVersion`, `command`, `status` — all strings).

### D4: Hand-written serialization (no serde derive)

**Decision**: The surface document is constructed as a `serde_json::Value` tree (or a `String` via manual formatting) rather than defining a Rust struct with `#[derive(Serialize)]`.
**Rationale**: The surface document is a single-output function with ~5 keys. Defining a struct hierarchy adds boilerplate (nested structs, doc comments on each field, conversion impls) that exceeds the value for this scale. A single `fn cli_surface() -> serde_json::Value` is simpler and keeps the serialization logic collocated with the argument parser.
**Risk**: If the CLI grows to 10+ commands, this function should be refactored to use typed structs. This is acceptable because the surface is still single-command.
**Sources**: Current codebase pattern — hand-written argument parsing (no clap), hand-written JSON output in `write_hotspots_json`.

### D5: `--help-json` with a command (e.g., `scryrs hotspots --help-json`) produces exit 2

**Decision**: `--help-json` is a **global** introspection flag alongside `--help` and `--version`. It describes the entire CLI surface, not a single command's help. If given after a command (e.g., `scryrs hotspots --help-json`), it currently exits 2 with "unknown argument" error because `hotspots` does not accept `--help-json` as an argument. In the future when per-command flags are supported, per-command `--help-json` could be added.
**Rationale**: The current parser doesn't support mixed command+flag parsing (it matched `[command, _path]` positionally). Adding per-command `--help-json` would require restructuring the parser. The global flag is sufficient for the v0 single-command surface — there is only one command, so global surface is per-command surface.
**Sources**: Current argument parser structure in `lib.rs`.

### D6: Surface document on stdout, not stderr

**Decision**: `--help-json` output goes to stdout, exit 0. Any errors writing stdout fall through to exit 1 (matching existing write-failure pattern).
**Rationale**: Consistent with `--help` and `--version` behavior. An agent that wants to capture the surface document can redirect stdout without stderr interference. The surface document is a successful result, not diagnostic information.
**Sources**: `--help` and `--version` both write to stdout and exit 0.

## Risks / Trade-offs

| Risk | Severity | Mitigation |
|------|----------|------------|
| R1: Surface document format goes stale as new commands are added. | Medium | `surfaceVersion` bumps on every change make staleness detectable. A `lastUpdated` or `generatedAt` field could be added if staleness becomes a real problem in practice. |
| R2: Hand-written serialization produces inconsistent JSON (whitespace, key ordering). | Low | `serde_json::Value` serialization is deterministic. Keys are inserted in code order (`serde_json::json!` macro preserves insertion order). Explicit test asserts full output shape. |
| R3: Agents rely on `--help-json` before it's widely adopted. | Low | The flag is new. No existing agents depend on it. The surface document documents its own version, so agents can detect when the format changes. |
| R4: `--help-json` with positional argument after `hotspots` produces confusing error (exit 2 instead of per-command surface). | Low | Documented in the contract. The v0 CLI doesn't support per-command flags. Adding per-command introspection is a future enhancement. |
| R5: `surfaceVersion` and `schemaVersion` both at `0.1.0` creates confusion about which to check. | Low | The surface document always contains both fields with clear documentation of each. Agents checking one or the other will still get correct behavior — they're different dimensions (CLI surface format vs command output format). |
