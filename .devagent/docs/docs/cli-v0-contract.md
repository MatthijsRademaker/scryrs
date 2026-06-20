# CLI v0 Contract

The v0 CLI surface for `scryrs` provides two commands: the `hotspots` placeholder and the `record` ingestion endpoint. This contract serves agent integrators and follow-up feature developers.

## Binary

**Name:** `scryrs`

## Commands

### `scryrs record --stdin | --file <PATH>`

Ingest JSONL trace events from stdin or a file. `--stdin` and `--file` are mutually exclusive; providing both or neither exits 2.

| Field | Value |
|-------|-------|
| Input | `--stdin` reads newline-delimited `TraceEvent` JSON from stdin; `--file <PATH>` reads from a JSONL file |
| Output | Single-line JSON summary on stdout; one JSON rejection diagnostic per rejected non-empty line on stderr |
| Exit 0 | All processed non-empty lines were accepted |
| Exit 1 | Ingestion completed but one or more non-empty lines were rejected; or I/O error writing output |
| Exit 2 | Fatal usage error (invalid mode, unreadable file, store failure) |

**Stdout summary envelope:**

```json
{"command":"record","schemaVersion":"0.1.0","accepted":5,"rejected":2}
```

- `command` is always `"record"`.
- `schemaVersion` matches `scryrs-types::SCHEMA_VERSION`.
- `accepted` and `rejected` are counts of processed non-empty lines.

**Stderr rejection diagnostics (one per rejected non-empty line):**

```json
{"line":3,"field":null,"reason":"expected value at line 1 column 1"}
```

- `line` is the 1‑based physical line number.
- `field` is `null` when the deserializer cannot determine a failing field, or a quoted field/path string.
- `reason` is a human-readable string describing the rejection.

**Ingestion behavior:** Blank or whitespace-only lines are skipped without incrementing accepted or rejected counts. Malformed JSON and schema-invalid `TraceEvent` lines are rejected with diagnostics, and ingestion continues with later lines.

**Persistence:** Accepted events are appended to `.scryrs/events.jsonl` (one JSON event per line) in the current working directory. This store is append-only and ingestion-only; no query, delete, or analysis APIs are provided.

### `scryrs hotspots <PATH>` (v0 placeholder)

The v0 placeholder command. Emits a versioned JSON envelope to stdout.

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
| 0 | Hotspots: JSON placeholder written successfully. Record: all processed non-empty lines were accepted. Help/version/surface display. |
| 1 | Record: one or more events rejected, or I/O error writing output. Hotspots: I/O error writing output. |
| 2 | Unknown commands, missing required arguments, invalid arguments, unsupported paths (usage errors). Record: fatal I/O error (unreadable file or store failure). |

All error messages and human-facing diagnostics are written to stderr.

## Agent-facing contract

### Hotspots command

**When to call:** An agent should call `scryrs hotspots <PATH>` when the agent needs scryrs' repository hotspot summary for a given local directory path.

**Input:** An explicit local directory path (required).

**Output:** A parseable JSON envelope on stdout (see envelope above). The agent can distinguish outcomes by exit code:

- Exit 0: JSON result available.
- Exit 2: Contract violation (missing PATH, unknown command, invalid args). Do not retry without fixing input.
- Exit 1: Transient runtime failure. May retry.

### Record command

**When to call:** An agent should call `scryrs record --stdin` to pipe JSONL `TraceEvent` data produced by hooks, or `scryrs record --file <PATH>` to ingest pre-recorded trace files.

**Input modes (mutually exclusive):**

- `--stdin`: Read newline-delimited `TraceEvent` JSON from stdin.
- `--file <PATH>`: Read JSONL from a file.

**Output:**

- Stdout: One JSON summary `{"command":"record","schemaVersion":"...","accepted":N,"rejected":M}`.
- Stderr: One JSON rejection diagnostic per rejected non-empty line (empty on 0 accept + 0 reject or when no rejections occur).

**Exit codes:**

- 0: All processed non-empty lines were accepted.
- 1: One or more rejected lines (ingestion continued).
- 2: Usage error (both/neither mode specified, unknown flags) or fatal I/O error (unreadable file, unwritable store).

**Fail-fast paths:** The following always exit 2 and write an error to stderr:

- Any command other than `hotspots` or `record` (including `components`, `trace`, `propose`, `graph`, `route`, `adapters`, `report`, `suggest-docs`)
- `scryrs hotspots` without a PATH argument
- `scryrs record` with neither or both input modes (mutually exclusive)
- `scryrs record --file` with an unreadable path
- `scryrs hotspots <FLAG>` / `scryrs record <FLAG>` — flags after a command fall through to the positional argument parser and are rejected as invalid arguments (no per-command introspection in v0)

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

## Local Development Testing

### Running tests

```bash
cargo test -p scryrs-cli
```

All tests for the `scryrs-cli` crate run through Cargo's built-in test runner. The crate includes:

- **Snapshot tests** (via `insta`) for `--help`, `--help-json`, and `hotspots` output — these verify exact output byte-for-byte against committed `.snap` files.
- **Identity tests** — verify `-h` produces identical output to `--help`, `-hj` to `--help-json`, and bare invocation to `--help`.
- **Error-path tests** — verify exit codes and error messages for unknown commands, missing arguments, and extra arguments.
- **Smoke tests** — exercise the public `run()` entrypoint to verify arg-collection wiring from the environment args iterator to the writer-based logic.

### Viewing snapshot diffs

When a snapshot test fails, Cargo prints a diff showing what changed between the expected (`.snap`) and actual output. The diff is human-readable and pinpoints every divergence — whitespace, wording, ordering, field presence.

### Updating snapshots

After an intentional change to the CLI contract (help text, `--help-json` surface document, or `hotspots` JSON envelope), update the committed snapshots:

```bash
# Batch-accept all new or changed snapshots:
cargo insta test --accept -p scryrs-cli

# Or review interactively (requires cargo-insta):
cargo insta review
```

### Installing cargo-insta

`cargo-insta` is optional — tests run and diff output works without it. It is only needed for the snapshot review/accept workflow:

```bash
cargo install cargo-insta
```
