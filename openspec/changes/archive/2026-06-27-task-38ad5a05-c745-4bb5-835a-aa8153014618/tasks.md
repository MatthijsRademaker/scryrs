## 1. Rewrite the CLI documentation page

- [x] 1.1 Rewrite the opening section of `cli-v0-contract.md` to start with a domain-driven overview explaining what scryrs CLI helps users accomplish, which problems it solves, and the two main workflow paths (local observe→detect loop and central live-ingest flow), with cross-links to `hotspots.md` and `trace-hook-contract.md` for deeper detail.
- [x] 1.2 Update the command count and enumeration from "five implemented commands" to six (`hotspots`, `record`, `hook`, `init`, `dashboard`, `server`), matching `help_text.rs` and `dispatch.rs`.
- [x] 1.3 Add a dedicated `scryrs hook` command section documenting: fail-open semantics (always exits 0, empty stdout), supported harnesses (`claude-code` via stdin, `pi` via `--file`), transport asymmetry, and the warning-log side channel at `.scryrs/hooks/<harness>-warnings.log`, matching `hook.rs` and `help_text.rs`.
- [x] 1.4 Add remote transport coverage to the `scryrs record` section: remote activation via `scryrs.json` remote section and `SCRYRS_REMOTE_*` env vars, config precedence (env → manifest → git fallback), no-dual-write/no-local-fallback semantics, the divergent remote output envelope (`transport`, `duplicate`, `failed` fields), and failure modes, matching `remote_config.rs`, `help_json.rs`, and `scryrs.json`.
- [x] 1.5 Add an agent-facing contract subsection for `hook` ("When to call" / "Input" / "Output" / "Exit codes") following the existing pattern used by `record`, `hotspots`, `dashboard`, and `server`.
- [x] 1.6 Correct the dashboard REST API table to include `GET /api/meta` and `GET /api/sessions/:sessionId` alongside `GET /api/hotspots`, `GET /api/sessions`, and `GET /api/events`, matching `scryrs-dashboard/src/server.rs` router and `dashboard.rs` help text.
- [x] 1.7 Remove `hook` from the "Out of scope for v0" section and replace the explicit future-command list (`components`, `trace`, `propose`, etc.) with a general statement that any unknown command exits 2 with a usage error.
- [x] 1.8 Update the global exit-code table to include hook's always-exit-0 behavior and remote record failure codes (missing remote identity, transport timeout, connection failure, non-2xx response, malformed response → exit 2).

## 2. Update navigation and cross-links

- [x] 2.1 Rename the nav label in `_nav.json` from `CLI v0 Contract` to `CLI Reference` while preserving the file slug `cli-v0-contract`.
- [x] 2.2 Verify cross-links from `hotspots.md` and `trace-hook-contract.md` to `cli-v0-contract.md` remain functional (they use the slug, not the label).
- [x] 2.3 Ensure the domain-driven opening includes bidirectional links to `hotspots.md` ("for hotspot interpretation and scoring rationale, see Hotspots") and `trace-hook-contract.md` ("for harness integration rules and fail-open guarantees, see Trace Hook Contract").

## 3. Verify documentation accuracy

- [x] 3.1 Verify the rewritten CLI page against `help_text.rs` — every command description, flag, argument, and output contract must match the current human-facing help text.
- [x] 3.2 Verify the rewritten CLI page against `help_json.rs` — the `hook` fail-open claim, remote record transport fields, config precedence, and dashboard endpoints must match the machine-readable surface document.
- [x] 3.3 Verify the rewritten CLI page against `dispatch.rs` — only six commands are listed as implemented and the unknown-command contract (exit 2) is accurate.
- [x] 3.4 Build the docs site (`cd .devagent/docs && pnpm run build`) to confirm no broken links, orphaned pages, or nav structure errors.
- [x] 3.5 Run `cargo test -p scryrs-cli` to confirm no snapshot test regressions (docs change should not affect snapshot output, but verify as a sanity check).
