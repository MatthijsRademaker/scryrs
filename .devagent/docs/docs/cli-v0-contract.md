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
| `-hj`, `--help-json` | Machine-readable CLI surface document to stdout | 0 |

Bare `scryrs` (no arguments) prints help to stdout and exits 0.

### `--help-json` surface document

The `--help-json` flag emits a versioned JSON document describing the complete CLI surface — commands, arguments, flags, output contracts, and exit codes — in a deterministic, machine-readable format.

**When to use:** An agent should call `scryrs --help-json` to discover the CLI surface before invoking commands. The document is idempotent: calling it repeatedly returns identical output.

**Output:** A single JSON object written to stdout. Stderr is empty. Exit code is 0.

**Top-level fields:**

| Field | Type | Description |
|-------|------|-------------|
| `surfaceVersion` | string | Semver version of the surface document format (independent of output envelope `schemaVersion`) |
| `binary` | string | Binary name (`"scryrs"`) |
| `commands` | array | Command entries, each with `name`, `description`, `arguments`, and `output` metadata |
| `globalFlags` | array | Flag entries, each with `name`, `short`, `long`, `description`, and `action` |
| `rootBehavior` | object | Describes bare-invocation behavior (`action`, `exitCode`) |
| `exitCodes` | object | Maps numeric exit codes (`"0"`, `"1"`, `"2"`) to their meanings |

**Command entry structure:** Each entry in `commands` includes:

- `name` (string): Command name
- `description` (string): Human-readable description
- `arguments` (array): Positional arguments, each with `name`, `type`, `required` (boolean), and `description`
- `output` (object): Output contract with `mimeType` and `fields` array (each field: `name`, `type`, `description`, `optional`)

**Versioning policy:** The `surfaceVersion` field follows semver:

- **Major bump:** Breaking changes to the surface document format (field renames, removals, structural changes)
- **Minor bump:** Additive changes (new commands, new flags, new fields)
- **Patch bump:** Clarifications (description text changes, documentation fixes)

Agents should check `surfaceVersion` before parsing to detect format changes. The surface version is independent of the output envelope's `schemaVersion`.

## Exit-code policy

| Code | Meaning |
|------|---------|
| 0 | Successful command, help display, version display, surface document display |
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
- `scryrs hotspots <FLAG>` — flags after a command fall through to the positional argument parser and are rejected as invalid arguments (no per-command introspection in v0)

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
