# Design — native harness hook command

## Context

Two harnesses, two fundamentally different hook models:

```
Claude Code                        Pi
───────────                        ──
subprocess hook                    in-process extension module
  OS spawns: <command>               Pi runtime imports default export
  event JSON on stdin                pi.on("tool_result", handler)
  exit 0 = allow                     handler returns; result unchanged
PRE-execution (no outcome yet)     POST-execution (has isError/outcome)
tool_name PascalCase               tool_name lowercase
  ("Read","Bash","WebSearch")        ("read","ast_grep_search","lsp_navigation")
```

Today both ship a JavaScript translator that maps the harness event to a canonical
`TraceEvent` and shells out to `scryrs record`. The Claude Code `.mjs` is doubly broken
(wrong settings.json shape; no runnable entry point — it never reads stdin or calls its
own export, so it is a silent no-op). The translation logic is duplicated across the two
JS/TS files and partially re-implemented in Rust (`record.rs` has its own
`collapse_newlines`).

## Goal

Make scryrs own harness translation natively. The hook command becomes `scryrs hook
<harness>`; translation lives once, in a Rust adapter crate. Claude Code drops JavaScript
entirely. Pi keeps the thinnest possible in-process shim its runtime requires.

## Decisions

### D1 — Dedicated `scryrs hook <harness>` subcommand (not a `record --format` flag)

`record`'s contract is "ingest canonical `TraceEvent` JSONL; exit 1/2 on bad input; print
a `{"command":"record",...}` summary." A harness hook needs the opposite contract: "accept
a foreign format; **never** fail the caller; emit nothing meaningful to the harness." These
are different error policies and different stdin schemas. Overloading `record` with
`--format claude-code` would fuse two contracts into one command. A `hook` subcommand whose
entire contract is the harness hook contract is the native-feeling design and generalizes
to future harnesses (`scryrs hook cursor`, …).

`hook` sits above `record`: it translates foreign → canonical, then reuses the same
`scryrs-core` `EventStore` persistence path `record` uses. Translation is new; persistence
is shared, not re-implemented. `record` stays as the low-level canonical-JSONL primitive.

### D2 — New `scryrs-adapter-harness` crate; one adapter per harness

The crate is the single source of truth for tool→event mapping. It exposes a small trait
along the lines of:

```
trait HarnessAdapter {
    /// Parse this harness's native event JSON into zero or one canonical TraceEvent.
    fn translate(&self, raw: &str, ctx: &HookContext) -> Result<Option<TraceEvent>, AdapterError>;
}
```

with `ClaudeCodeAdapter` and `PiAdapter` implementations. Shared helpers (newline collapse,
schema version, envelope construction) live in the crate, removing the JS/Rust duplication.
`Option<TraceEvent>` lets an adapter return `None` for tools it does not track (pass-through),
mirroring today's whitelist behavior.

> **Naming (decided):** the crate is `scryrs-adapter-harness`. The existing `scryrs-adapter-*`
> crates adapt scryrs data *outward* to doc formats (markdown, rspress) while this one adapts
> harness events *inward*; the directional difference is accepted in exchange for keeping the
> `scryrs-adapter-*` family prefix consistent across the workspace.

### D3 — Per-harness translation differences the adapter must encode

| Concern            | claude-code adapter                          | pi adapter                                  |
|--------------------|----------------------------------------------|---------------------------------------------|
| tool_name casing   | PascalCase (`Read`, `Bash`, `WebSearch`)     | lowercase (`read`, `ast_grep_search`)       |
| timing             | pre-execution                                | post-execution                              |
| outcome            | always `Success` (result unknown pre-exec)   | `isError` → `Failure`/`Success`             |
| tracked tools      | Read, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, WebFetch (Bash debug-gated) | read, ast_grep_search, lsp_navigation, edit, write (Bash debug-gated) |
| special mappings   | Read→FileOpened, Grep/Glob/WebSearch→SearchRun, Edit/Write/NotebookEdit→EditMade, WebFetch→DocRetrieved | lsp_navigation→SymbolInspected, or FailedLookup on error |

The PascalCase point is a real, previously-latent bug: the dead `.mjs` lowercased and
matched `web_search`/`web_fetch` with underscores, so `"WebSearch".toLowerCase()` =
`"websearch"` would never have matched. The Rust adapter matches the documented PascalCase
names directly.

### D4 — Fail-open is the hook command's defining contract

scryrs must NEVER block harness execution. For the `hook` subcommand this means: any error
— malformed JSON, unknown harness event, store locked, panic-worthy condition — results in
**exit 0** with no harness-visible output, plus a timestamped line in the existing fail-open
warning log (`.scryrs/hooks/<harness>-warnings.log`). This is deliberately the inverse of
`record`'s 1/2 exit policy, and is the second reason `hook` is its own command (D1).

Claude Code's documented contract confirms this is trivial: **exit 0 with no stdout is
sufficient to allow the tool**. The `.mjs`'s `{continue:true}` stdout payload is not even
the documented shape and is unnecessary for an observer. The native hook writes nothing and
exits 0.

One contract gap worth stating: with the `.mjs`, node's `spawn` caught a missing `scryrs`
binary (ENOENT) and fail-opened. With a native command, if `scryrs` itself is missing there
is no process to fail-open — but Claude Code treats a missing/erroring hook command as
non-fatal and proceeds, so the net effect is still fail-open. Net failure surface shrinks
because node is removed from the path.

### D5 — Read identity and store location from the payload, not the environment

The Claude Code PreToolUse payload includes `session_id`, `cwd`, `transcript_path`,
`permission_mode`, `tool_name`, `tool_input`. The adapter reads `session_id` from the
payload (the `.mjs` guessed it from `CLAUDE_SESSION_ID`-style env vars). Because
`CANONICAL_STORE_PATH` (`.scryrs/scryrs.db`) is **cwd-relative**, the hook resolves the
store against the payload `cwd` rather than relying on the spawned process's working
directory. More robust than `process.cwd()`.

### D6 — Input transport: stdin for Claude Code, `--file` for Pi

`scryrs hook <harness>` accepts input via stdin by default and `--file <path>` as an
alternative (mirroring `record`'s dual-input design). Claude Code pipes the event on stdin.
Pi's `exec()` opens stdin as `/dev/null` (`index.ts:7,220`), so the Pi shim writes the raw
event to a temp file and calls `scryrs hook pi --file <tmp>`.

### D7 — Pi keeps a thin shim; Claude Code keeps nothing

Pi's runtime loads an in-process module and calls `pi.on(...)`; there is no subprocess hook
for Pi to invoke the way Claude Code does. So `hooks/pi/index.ts` cannot be deleted. It is
reduced to: register `session_start`/`tool_result`, resolve `session_id` from Pi's
`SessionManager`, serialize the raw event, hand it to `scryrs hook pi --file`. The large
tool→event `switch` and mapping helpers (≈ `index.ts:183-449`) are removed — that logic now
lives in the Rust pi adapter. Claude Code's `.mjs` is deleted outright.

## Architecture

```
                          ┌──────────────────────────────────────────────┐
                          │            scryrs-adapter-harness              │
                          │  HarnessAdapter trait                          │
   Claude Code ───────────▶  ClaudeCodeAdapter  (PascalCase, pre-exec)    │
   scryrs hook claude-code │  PiAdapter          (lowercase, post-exec)    │
   (event JSON on stdin)   │  shared: envelope, newline collapse, schema   │
                          └───────────────────┬──────────────────────────┘
   Pi (thin index.ts) ─────────────────────────┘   │ TraceEvent
   scryrs hook pi --file <tmp>                       ▼
                                          scryrs-core EventStore.append
                                          (store resolved against payload cwd)
                                                     │
                                          fail-open: any error → exit 0 + warning log
```

## Risks / trade-offs

- **Pi exec stdin = /dev/null** forces the temp-file path for Pi; the shim must clean up
  temp files and fail-open on write errors (already true today).
- **Adapter naming** clashes directionally with existing doc adapters (D2) — cosmetic.
- **No migration** is intentional (sole user). Stale `.mjs`-based settings.json must be
  re-initialized; they captured nothing, so there is no data loss.
- **Tool-name drift**: Claude Code tool names are matched as documented PascalCase; if
  upstream renames a tool the adapter misses it (fail-open → silent gap). Verification
  fixtures pin the expected names.

## Verification plan

End-to-end, per the user's request:

1. Pipe a real Claude Code PreToolUse payload (`{"session_id":…,"cwd":…,"tool_name":"Read","tool_input":{"file_path":…}}`) into `scryrs hook claude-code`; assert exit 0, no stdout, and the event landed in the `EventStore` under the payload `cwd`.
2. Force errors (malformed JSON; unwritable store) and assert exit 0 + warning-log line — never a non-zero exit.
3. Pi: write a `tool_result` event to a temp file, call `scryrs hook pi --file`, assert the mapped event (including `isError`→`Failure` and `lsp_navigation`→`SymbolInspected`/`FailedLookup`).
4. Unit tests in `scryrs-adapter-harness` for every tool→event mapping in both adapters, including PascalCase matching and pass-through (`None`) for untracked tools.
