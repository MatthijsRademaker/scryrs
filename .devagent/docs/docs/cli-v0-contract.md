# CLI v0 Contract

The v0 CLI surface for `scryrs` is frozen to exactly one placeholder command. This contract serves agent integrators and follow-up feature developers.

## Binary

**Name:** `scryrs`

## Commands

### `scryrs hotspots <PATH>` (v0 placeholder)

The sole v0 command. Emits a versioned JSON envelope to stdout.

| Field | Value |
|-------|-------|
| Input | Required local directory `<PATH>` |
| Output | Versioned JSON on stdout |
| Exit 0 | JSON written successfully |
| Exit 2 | PATH argument omitted (usage error on stderr) |

**JSON envelope:**

```json
{"schemaVersion":"0.1.0","command":"hotspots","status":"placeholder"}
```

- `schemaVersion` matches `scryrs-types::SCHEMA_VERSION`.
- `command` is always `"hotspots"`.
- `status` is always `"placeholder"` (no engine behavior in v0).

No human-readable text is emitted to stdout for this command. Agents should parse stdout as JSON.

## Global flags

| Flag | Behavior | Exit code |
|------|----------|-----------|
| `-h`, `--help` | Help text to stdout | 0 |
| `-V`, `--version` | Version string to stdout | 0 |

Bare `scryrs` (no arguments) prints help to stdout and exits 0.

## Exit-code policy

| Code | Meaning |
|------|---------|
| 0 | Successful command, help display, version display |
| 2 | Unknown commands, missing required arguments, invalid arguments, unsupported paths (usage errors) |
| 1 | Unexpected runtime failures (I/O errors, internal panics) |

All error messages and human-facing diagnostics are written to stderr.

## Agent-facing contract

**When to call:** An agent should call `scryrs hotspots <PATH>` when the agent needs scryrs' repository hotspot summary for a given local directory path.

**Input:** An explicit local directory path (required).

**Output:** A parseable JSON envelope on stdout (see envelope above). The agent can distinguish outcomes by exit code:

- Exit 0: JSON result available.
- Exit 2: Contract violation (missing PATH, unknown command, invalid args). Do not retry without fixing input.
- Exit 1: Transient runtime failure. May retry.

**Fail-fast paths:** The following always exit 2 and write an error to stderr:

- Any command other than `hotspots` (including `components`, `trace`, `propose`, `graph`, `route`, `adapters`, `report`, `suggest-docs`)
- `scryrs hotspots` without a PATH argument

## Out of scope for v0

The following commands are **not** defined in the v0 contract and will exit 2 with a usage error if invoked:

- `components`
- `trace`
- `propose`
- `graph`
- `route`
- `adapters`
- `report`
- `suggest-docs`

These names align with the vision document's future command vocabulary but are not part of the v0 surface.
