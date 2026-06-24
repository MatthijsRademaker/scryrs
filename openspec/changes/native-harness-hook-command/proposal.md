## Why

The Claude Code reference hook does not work. Two independent defects stack so that nothing is captured, yet nothing visibly breaks:

1. **`init --agent claude-code` emits an invalid settings.json shape.** It instructs the user to write `"hook": "<string>"` (`init.rs:46-55`, `:159`). The native Claude Code contract is `"hooks": [{ "type": "command", "command": "<string>" }]`. A literal copy-paste of scryrs's own instructions registers zero hooks.
2. **`scryrs-hook.mjs` has no runnable entry point.** It is written against an in-process module-import contract (`export default async function(input)`, `scryrs-hook.mjs:343`) but Claude Code `command` hooks invoke a subprocess and pass the event on **stdin**. Running `node scryrs-hook.mjs` loads the module, defines the export, and exits 0 having done nothing — a silent no-op that fail-opens by accident.

The `.mjs` is, in substance, only a translator: it maps the harness tool event to a canonical `TraceEvent` and then shells out to `scryrs record --stdin`. Everything else it does (newline collapse, schema, persistence) already lives in Rust — sometimes duplicated. The translation is the only thing scryrs does not yet own natively.

This change removes the JavaScript transport for Claude Code entirely and bakes harness translation into scryrs as a first-class `scryrs hook <harness>` subcommand. No node dependency, one process per tool-use instead of two, and a single Rust source of truth for tool→event mapping. The Pi harness keeps a minimal in-process shim (its runtime mandates one) but delegates all translation to the same Rust adapter.

There is a single user today. This change takes **hard breaking changes** with no backwards compatibility.

## What Changes

- **BREAKING**: Delete `hooks/claude-code/scryrs-hook.mjs` and its `include_str!()` embedding. Claude Code no longer installs or depends on any JavaScript or node runtime.
- **BREAKING**: Add a `scryrs hook <harness>` subcommand whose contract IS the harness hook contract — accept a foreign harness event, translate it, persist it, and **never block harness execution** (always exit 0, fail-open).
- **BREAKING**: Add a new `scryrs-adapter-harness` crate holding the single source of truth for harness-event → canonical `TraceEvent` translation, with a `claude-code` adapter and a `pi` adapter.
- **BREAKING**: `init --agent claude-code` now creates-or-merges `.claude/settings.json` with the native `"hooks": [{ "type": "command", "command": "scryrs hook claude-code" }]` block, instead of writing a `.mjs` and printing hand-edit instructions. The settings.json collision-refusal behavior is replaced with idempotent merge.
- **BREAKING**: `hooks/pi/index.ts` is reduced to a thin transport that forwards the raw Pi `tool_result`/`session_start` event to `scryrs hook pi`; all tool→event mapping moves out of TypeScript into the Rust adapter.
- The Claude Code adapter reads `session_id` and `cwd` directly from the PreToolUse payload (no env-var guessing) and resolves the trace store relative to that `cwd`.
- Update verification fixtures to exercise the native subcommand path for both harnesses.

## Capabilities

### New Capabilities

- `harness-hook-command`: Define the `scryrs hook <harness>` subcommand — discoverability, stdin/`--file` input, per-harness routing, and the unconditional fail-open (never block the harness) contract.
- `harness-adapter-crate`: Define the `scryrs-adapter-harness` crate (name decided) as the single source of truth translating each harness's native tool event into a canonical `TraceEvent`.

### Modified Capabilities

- `claude-code-reference-hook`: Replace the JavaScript `.mjs` transport with the native `scryrs hook claude-code` subcommand. Tool mapping moves to Rust; tool names are matched as PascalCase; fail-open is "exit 0"; `session_id`/`cwd` come from the payload.
- `pi-reference-hook`: Reduce the Pi extension to a thin transport that delegates translation to `scryrs hook pi`.
- `init-installer`: Claude Code install creates-or-merges `.claude/settings.json` with the native command and installs no `.mjs`; Pi install writes the slimmed `index.ts`.
- `init-verification`: Validate the create/merge settings.json behavior and drive the native subcommand path for both harnesses.
- `cross-harness-verification`: Exercise `scryrs hook claude-code` (stdin) and `scryrs hook pi` end-to-end, including fail-open.
- `trace-hook-contract`: Document the two per-harness transport models (native command for Claude Code; in-process shim for Pi) and `scryrs hook` as the harness integration entry point above `scryrs record`.
- `scryrs-manifest`: Drop the Claude Code `.mjs` reference source; describe the native `scryrs hook claude-code` command and PascalCase intercepted tools; describe the Pi shim's delegation.

## Impact

- **Affected code**: new `crates/scryrs-adapter-harness/`; `crates/scryrs-cli/` (new `hook` subcommand, dispatch, help, help-json, `init.rs` create/merge); `hooks/pi/index.ts` (slimmed); deleted `hooks/claude-code/scryrs-hook.mjs`; verification scripts/fixtures; `scryrs.json` manifest; hook READMEs and `.devagent/docs/`.
- **Behavioral impact**: Claude Code trace capture works for the first time. One fewer process per tool-use and no node cold-start on the hot path. No runtime node dependency for Claude Code.
- **Migration**: None provided by design. Existing installs (settings.json pointing at `node …mjs`) must re-run `init`. No data loss — the old `.mjs` captured nothing.
- **Asymmetry (intentional)**: Claude Code's transport is fully native (`.mjs` deleted). Pi's runtime loads an in-process module, so a minimal `index.ts` shim remains; only its translation logic is removed. Symmetry holds at the command (`scryrs hook <harness>`) and translation (one crate) layers, not at the transport layer.
