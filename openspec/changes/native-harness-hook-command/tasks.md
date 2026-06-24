## 1. Create the harness adapter crate

- [x] 1.1 Add `crates/scryrs-adapter-harness/` to the workspace `members` in `Cargo.toml`.
- [x] 1.2 Define a `HarnessAdapter` trait: `translate(raw, ctx) -> Result<Option<TraceEvent>, AdapterError>`, plus a `HookContext` carrying `session_id` and resolved store path.
- [x] 1.3 Implement `ClaudeCodeAdapter`: parse PreToolUse JSON; match PascalCase tool names; map Read→FileOpened, Grep/Glob/WebSearch→SearchRun, Edit/Write/NotebookEdit→EditMade, WebFetch→DocRetrieved; Bash→CommandExecuted debug-gated; outcome always `Success`; untracked → `None`.
- [x] 1.4 Implement `PiAdapter`: parse Pi `tool_result`/`session_start` JSON; match lowercase tool names; map read→FileOpened, ast_grep_search→SearchRun, edit/write→EditMade, lsp_navigation→SymbolInspected (or FailedLookup when `isError`); Bash debug-gated; `isError`→`Failure`/`Success`; untracked → `None`.
- [x] 1.5 Move shared `collapse_newlines`, schema-version constant, and `TraceEvent` envelope construction into the crate; delete the duplicated copy in `record.rs` if it becomes shareable.
- [x] 1.6 Unit-test every mapping in both adapters: tool coverage, PascalCase/lowercase matching, pass-through `None`, Bash debug-gating, Pi `isError` and `lsp_navigation` branches.

## 2. Add the `scryrs hook <harness>` subcommand

- [x] 2.1 Add `hook` to the pre-clap command whitelist and clap subcommand definition: positional `<harness>` (`claude-code`|`pi`), `--stdin` (default) and `--file <path>` (mutually exclusive), reusing the `record` input pattern.
- [x] 2.2 Implement `execute_hook`: read input, resolve store path from payload `cwd` (fallback `CANONICAL_STORE_PATH`), select adapter by harness, translate, and `EventStore.append` the event when `Some`.
- [x] 2.3 Implement the fail-open contract: ANY error (bad JSON, unknown harness event, store error) → exit 0, no stdout, timestamped line in `.scryrs/hooks/<harness>-warnings.log`. Never exit non-zero.
- [x] 2.4 Honour `SCRYRS_DEBUG` for Bash gating and for the existing debug-log breadcrumbs, consistent with `record`.
- [x] 2.5 Surface `hook` in `write_help()` and `--help-json` (bump `SURFACE_VERSION`); add dispatch + golden tests.

## 3. Rework Claude Code installation in init

- [x] 3.1 Remove the `claude-code` `include_str!()` embedding and registry source asset; delete `hooks/claude-code/scryrs-hook.mjs`.
- [x] 3.2 Change `init --agent claude-code` to create-or-merge `.claude/settings.json` with `"hooks": { "PreToolUse": [{ "matcher": "", "hooks": [{ "type": "command", "command": "scryrs hook claude-code" }] }] }`, preserving unrelated existing keys and being idempotent on re-run.
- [x] 3.3 Replace the settings.json collision-refusal path with merge; update next-step text to drop the `.mjs` and hand-edit instructions.
- [x] 3.4 Keep the source-repo self-install refusal for `claude-code`.

## 4. Slim the Pi extension to a transport shim

- [x] 4.1 Reduce `hooks/pi/index.ts` to: register `session_start`/`tool_result`, resolve `session_id` from Pi `SessionManager`, write the raw event to a temp file, call `scryrs hook pi --file <tmp>`, clean up, fail-open.
- [x] 4.2 Remove the tool→event `switch` and mapping helpers from `index.ts` (now owned by the Rust pi adapter).
- [x] 4.3 Confirm `init --agent pi` still installs the slimmed `index.ts` to `.pi/extensions/pi-trace/index.ts`; update embedded source.

## 5. Verification

- [x] 5.1 E2E: pipe a real PreToolUse payload into `scryrs hook claude-code`; assert exit 0, no stdout, event persisted under payload `cwd`.
- [x] 5.2 E2E: malformed JSON and unwritable store both exit 0 and append a warning-log line.
- [x] 5.3 E2E: `scryrs hook pi --file` maps a `tool_result` correctly, including `isError`→`Failure` and `lsp_navigation` branches.
- [x] 5.4 Update `cross-harness-verification` fixtures/scripts to drive the native subcommand path for both harnesses; retire `.mjs`-based fixtures.
- [x] 5.5 Update `init-verification` fixtures for the create/merge settings.json behavior.
- [x] 5.6 Run `scripts/precommit-run` and confirm fmt/check/clippy/test pass.

## 6. Docs and manifest

- [x] 6.1 Update `scryrs.json` harness metadata to reflect the native `scryrs hook` command for Claude Code and the thin Pi shim.
- [x] 6.2 Update `hooks/claude-code/README.md` (now command-based, no `.mjs`/node) and `hooks/pi/README.md` (thin shim).
- [x] 6.3 Update `.devagent/docs/` trace-hook-contract and roadmap entries to document native subcommand transport for Claude Code and the Pi shim asymmetry.

## 7. Spec deltas (authored)

- [x] 7.1 `init-verification` spec delta (create/merge contract replaces non-mutating refusal).
- [x] 7.2 `trace-hook-contract` spec delta (per-harness transport models; `scryrs hook` above `record`).
- [x] 7.3 `scryrs-manifest` spec delta (native command metadata, PascalCase tools, Pi shim).
