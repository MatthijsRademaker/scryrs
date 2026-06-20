## Why

The v0 CLI contract is frozen and implemented (single `hotspots` placeholder command, `--help`, `--help-json`, `--version`, exit codes 0/1/2, snapshot-tested output). But a first-time user who clones the repository finds no quickstart guidance, an empty `examples/` directory, and a README that describes workspace layout without showing how to build or run anything. The CLI works — but there is no path from "cloned the repo" to "ran the first command" without reading source code or the developer docs. This is the last gap between "the CLI exists technically" and "a human can discover and use it."

Now is the right time because the CLI surface is stable, snapshot tests lock the exact output, and no engine behavior is wired yet — so the quickstart can honestly document the current placeholder state without fear of immediately going stale.

## What Changes

1. **Add a Quickstart section to `README.md`** that walks a first-time user from clone to first command: prerequisites, build from source, run `--help`, run `hotspots`, expected output, and troubleshooting.

2. **Create example files in `examples/`** that are copy-paste runnable shell commands covering the full CLI surface (help, version, help-json, hotspots placeholder, error paths).

3. **Add a "current limitations" section** in the quickstart that honestly documents: only one command exists, output is a placeholder, no engine behavior is wired, and which future features are out of scope.

4. **Keep everything aligned with snapshot-tested CLI output** — all example commands and expected outputs are verified against the existing `insta` snapshots (no drift between docs and behavior).

## Capabilities

### New Capabilities

- `cli-quickstart`: Quickstart documentation in README enabling first-time users to build, run, and understand scryrs from a freshly cloned repository without reading source code or developer docs.
- `cli-examples`: Copy-paste runnable shell examples demonstrating all CLI surface commands (help, version, help-json, hotspots placeholder invocation, and error-path diagnostics).

### Modified Capabilities
<!-- No existing capability requirements change — this is purely additive documentation. -->

## Impact

- **Code changes**: None. No Rust code, argument parsing, CLI contract, or test behavior changes.
- **Documentation changes**: `README.md` updated with Quickstart and Current Limitations sections. New example files at `examples/` directory.
- **No contract changes**: CLI output, exit codes, error messages, and help text remain unchanged. Snapshot tests continue to pass as-is.
- **Examples align with test behavior**: Every command shown in the quickstart and examples has corresponding snapshot tests verifying its exact output.
