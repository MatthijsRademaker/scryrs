# Verify Init Command

## Why

The `scryrs init` command currently has two verification gaps and one documentation conflict that prevent claiming it provisions a working scry setup:

1. **Orphan verification:** `scripts/verification/installed-hook-e2e.mjs` correctly tests init-installed consumer artifacts but is not wired into the authoritative Docker-backed `scripts/verify-trace-capture` lane. The existing source-hook fixtures (claude-code-e2e.mjs, pi-hook-e2e.mjs) load artifacts from `hooks/` in the source tree — they prove reference hooks work, not that init-installed consumer artifacts work.

2. **Schema conflict:** The Claude Code `.claude/settings.json` hook configuration schema emitted by the installer (nested `"hooks"/"PreToolUse"/"type":"command"` command-block form) differs structurally from the schema documented in `hooks/claude-code/README.md` (flat `"hook": "node ..."` string form). Users following the README get a different config than what the installer instructs, creating ambiguity about what constitutes a correct consumer setup.

3. **Stale documentation:** `scripts/verification/README.md` lists only `claude-code-e2e.mjs` and `pi-hook-e2e.mjs` in its fixture tree, even though `installed-hook-e2e.mjs` exists. The `test-coverage-lane/spec.md` already requires installed-hook end-to-end validation, but this requirement is currently unmet.

## What Changes

### Wire installed-hook e2e into the authoritative verification lane

- Add `scripts/verification/installed-hook-e2e.mjs` as a third fixture phase in `scripts/verify-trace-capture`, gated behind an `--init-only` flag (matching the existing `--claude-only` / `--pi-only` filter pattern).
- The installed-hook phase runs after the existing source-hook fixtures when invoked without flags (full lane), or independently via `--init-only`.
- The phase runs `scryrs init --agent claude-code` and `scryrs init --agent pi` in temporary consumer directories, loads installed artifacts from those consumer paths, exercises tool-capture forwarding against the real `scryrs` binary, and proves event persistence via `scryrs hotspots`.

### Reconcile Claude Code settings.json schema

- Audit which `.claude/settings.json` hook configuration form is canonical for current Claude Code versions: the flat `"hook"` string form documented in `hooks/claude-code/README.md`, or the nested `"type":"command"` command-block form emitted by `init.rs`.
- Update the non-canonical source (either `init.rs` next-steps text or `hooks/claude-code/README.md`) to match the canonical form.
- If both forms are intentionally supported, document this explicitly in both locations.

### Update verification documentation

- Add `installed-hook-e2e.mjs` to the fixture tree in `scripts/verification/README.md` with a description of its purpose relative to the source-hook fixtures.
- Update `scripts/verify-trace-capture` usage documentation to include the new `--init-only` flag.

### Keep the init installer contract unchanged

- The init installer remains non-mutating for `.claude/settings.json` (no auto-creation, no overwrite).
- No changes to hook business logic, TraceEvent schema, or any Rust crate outside the verification scripts and documentation.
- The installed-hook e2e verification explicitly validates the deterministic next-step text that tells users how to complete setup manually, rather than implying zero-touch Claude Code setup.

## Impact

- **Affected specs:** `openspec/specs/test-coverage-lane/spec.md` — the installed-hook e2e requirement becomes satisfied by wiring the script into the verification lane.
- **Affected code:** `scripts/verify-trace-capture` (add `--init-only` flag and third fixture phase), `scripts/verification/README.md` (update fixture tree). Optionally `crates/scryrs-cli/src/init.rs` or `hooks/claude-code/README.md` (schema reconciliation — one must change).
- **Affected docs:** `scripts/verification/README.md`, `hooks/claude-code/README.md` if it is the non-canonical schema source.
- **No impact on:** Rust crates (`scryrs-cli`, `scryrs-types`, `scryrs-core`), hook business logic, TraceEvent schema, existing CLI commands (`hotspots`, `record`), existing unit tests, CI configuration (verify-trace-capture lane assignment remains as-is — gating this behind `--init-only` avoids changing CI trigger policy).
- **Risk:** Adding installed-hook e2e to verify-trace-capture increases runtime (npm install tsx for Pi, async event persistence waits). The existing lane takes ~41s with cached build; the addition is estimated at 60-90s, keeping total runtime well under the 3-minute PR-gate threshold set in `test-coverage-lane/spec.md`.