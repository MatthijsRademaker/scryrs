---
name: pi-hook-changes
description: "Recipe for changing the Pi trace hook and its install wiring. Use when: the task mentions hooks/pi, the Pi extension, trace hook install, scryrs init --agent pi, or .pi/extensions. Do not use for: the Rust-side event translation in crates/scryrs-adapter-harness (that's regular crate work — see crate-map) or general testing (testing-this-repo)."
metadata:
  curated: true
  sources:
    - hooks/**
    - crates/scryrs-cli/src/init.rs
    - scripts/verify-trace-capture
    - scripts/verification/**
  last-verified: 2026-07-04
  verified-commit: 8f8ca6cdcee59fb31b4a12f5ab8c48787f2c345c
---

# Pi Hook Changes

The hook is a **transport-only shim**: `hooks/pi/index.ts` (~160 lines) forwards raw Pi events to `scryrs hook pi --file <tmp>`, fail-open, no translation logic. All tool→TraceEvent mapping lives in Rust (`crates/scryrs-adapter-harness`). If your change adds mapping, schema, or HTTP to the shim, you're changing the wrong layer.

**Ownership rule (hard):** `hooks/pi/index.ts` is the ONLY editable source. There are two copies you must never edit:

- `.pi/extensions/scryrs/index.ts` — the installed runtime copy (gitignored, may be absent). Refresh it; never edit it. <!-- skill-lint-ignore: gitignored install target, absent until scryrs init runs -->
- `.pi/extensions/pi-trace/index.ts` — a **stale, git-tracked decoy**: a 478-line old hook still using the retired `scryrs record --file` contract. It is not the install target and not the canonical source. Do not edit it, and do not copy patterns from it.

## Read first

1. `hooks/pi/index.ts` — the canonical shim: session_start/tool_result handlers, `forwardRawEvent()`, fail-open try/catch.
2. `hooks/pi/README.md` — the transport design, the Pi tool→event mapping table, and install/verification commands.

## Steps

1. Confirm you're on the canonical file, not a copy.
   Verify: `grep -n "scryrs hook pi" hooks/pi/index.ts` — hit present (the decoy uses `scryrs record` instead).
2. Edit `hooks/pi/index.ts`. Keep it transport-only and fail-open; if you use a new Pi runtime API, extend `hooks/pi/ambient.d.ts` to match.
   Verify: `grep -nE "fetch\(|XMLHttpRequest" hooks/pi/index.ts` — zero hits (a Rust test enforces this no-HTTP invariant and will fail CI).
3. Remember the shim is **compiled into the binary**: `crates/scryrs-cli/src/init.rs` embeds it via `include_str!("../../../hooks/pi/index.ts")`. Your edit does nothing at runtime until the binary is rebuilt.
   Verify: `grep -n "include_str" crates/scryrs-cli/src/init.rs` — the embed line is intact.
4. Run the Docker-backed E2E (rebuilds the binary inside Docker; no host toolchain): `scripts/verify-trace-capture --pi-only`. For install-path changes also run `--init-only`.
   Verify: script exits 0.
5. Update the contract docs in lockstep — behavioral changes must land in `.devagent/docs/docs/trace-hook-contract.md` (transport table, reference-hooks section) and `hooks/pi/README.md` (mapping table).
   Verify: `grep -n "scryrs hook pi" .devagent/docs/docs/trace-hook-contract.md hooks/pi/README.md` — claims still match your changed behavior.
6. Refresh the local dogfooding install: delete `.pi/extensions/scryrs/index.ts` and re-run `scryrs init --agent pi` (needs the rebuilt binary). <!-- skill-lint-ignore: gitignored install target -->
   Verify: `diff hooks/pi/index.ts .pi/extensions/scryrs/index.ts` — identical (or the install dir is absent and you note init wasn't run in this environment). <!-- skill-lint-ignore: gitignored install target -->

## Easy-to-miss details

- **Rebuild-before-refresh is the #1 miss**: edit → forget the `include_str!` embed → re-run init with the OLD binary → installed copy silently stays old. Always rebuild (the verify script does) before refreshing.
- **Event/schema changes are Rust-side concerns**: what events are emitted is decided in `crates/scryrs-adapter-harness` and versioned by `SCHEMA_VERSION` in `crates/scryrs-types/src/lib.rs` — plus the `trace-event-schema` contract spec. The shim edit alone can't change the wire format.
- **Claude Code is different plumbing**: `hooks/claude-code/` is README-only; that harness uses the native `scryrs hook claude-code` command merged into `.claude/settings.json` — there is no JS hook file to edit for it.
- **`scryrs doctor` checks the installed copy** (`pi_hook_finding` in `crates/scryrs-cli/src/doctor.rs`) — useful to confirm install state after refresh.

## Self-check before reporting done

- [ ] Only `hooks/pi/index.ts` (+ `ambient.d.ts`/README) edited — neither `.pi/extensions/` copy touched?
- [ ] No `fetch`/`XMLHttpRequest` introduced?
- [ ] `scripts/verify-trace-capture --pi-only` green?
- [ ] `trace-hook-contract.md` and `hooks/pi/README.md` still true after your change?
