## Why

The v0 CLI contract is frozen — exactly one command (`hotspots`), deterministic JSON output, documented exit codes. But the contract is only discoverable by parsing prose help text (`--help`). An LLM agent cannot programmatically enumerate the CLI surface: it cannot ask "what commands exist?" and receive a structured answer, cannot ask "what arguments does `hotspots` take?" without reading English prose, and cannot confirm output shape or exit code semantics without invoking commands speculatively and inferring from failures. This is the last gap between "human-readable CLI" and "agent-invokable tool." Adding a deterministic machine-readable surface now, while the CLI is still single-command, establishes the pattern before multi-command expansion makes it harder.

## What Changes

1. **Add `scryrs --help-json` flag** that outputs a single JSON object describing the complete CLI surface: binary name, commands (with arguments, types, descriptions), global flags, root behavior, exit code table, and the schema version of the description format itself.

2. **Emit the surface document to stdout**, exit 0 — consistent with `--help` and `--version` behavior.

3. **Include output contract metadata** in the surface description — each command's output shape (MIME type and a lightweight field-level schema), so agents know what to parse before invoking.

4. **Version the CLI surface description** independently of the command output's `schemaVersion` — a `surfaceVersion` field on the root object that evolves as the CLI gains commands.

5. **Document the `--help-json` contract** in the CLI contract design note (`cli-v0-contract.md`) — what it produces, what each field means, and how agents should use it.

6. **Do not change** any existing behavior — `--help`, `--version`, `hotspots <PATH>`, exit codes, error messages, bare invocation, and all tests must pass unchanged.

## Capabilities

### New Capabilities

- `cli-machine-surface`: Structured JSON description of the CLI surface, enumerating commands, arguments, flags, output contracts, and exit codes in a versioned, deterministic format discoverable via `--help-json`.

### Modified Capabilities

- (none — no existing capability changes requirements)

## Impact

- **Code changes**: `crates/scryrs-cli/src/lib.rs` only — new argument arm for `--help-json`, new function to serialize the surface document, updated test suite.
- **No contract changes**: Exit codes, JSON output format, command surface, error messages, and help text remain identical. The existing v0 contract is extended, not modified.
- **No engine crate changes**: `scryrs-types` and all other workspace crates untouched.
- **Docs change**: `.devagent/docs/docs/cli-v0-contract.md` updated with `--help-json` contract documentation.
- **Test additions**: New tests for `--help-json` output shape, field presence, exit code 0, error behavior when combined with commands.
