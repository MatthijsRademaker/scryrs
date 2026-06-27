## Why

The CLI documentation page (`cli-v0-contract.md`) is stale and architecture-focused rather than domain-driven. It claims "five implemented commands" but the executable surface has six (`hotspots`, `record`, `hook`, `init`, `dashboard`, `server`). The `hook` command — a user-facing harness integration entry point with fail-open semantics — is entirely undocumented. The `record` section covers local persistence only, omitting the implemented remote transport mode. The dashboard endpoint table lists three routes when the router serves five. The page opens with a technical contract framing and never explains what problem scryrs CLI solves for users or which workflows it enables.

These gaps make the docs unreliable for discovery and implementation. Readers encounter a contract-first information architecture that hides the user value proposition behind low-level detail.

## What Changes

- **Rewrite `cli-v0-contract.md`** as a single domain-driven reference page with a workflow-first opening, a command-choosing guide, and refreshed command contract sections.
- **Add a domain-driven overview** explaining what scryrs CLI helps users accomplish, which problems it solves, and the two main workflow paths (local observe→detect loop and central live-ingest flow).
- **Document all six implemented commands** (`hotspots`, `record`, `hook`, `init`, `dashboard`, `server`) with dedicated sections, accurate to `help_text.rs`, `help_json.rs`, and `dispatch.rs`.
- **Add a dedicated `hook` section** documenting fail-open semantics (always exits 0, empty stdout), supported harnesses (`claude-code` with stdin, `pi` with `--file`), and the warning-log side channel at `.scryrs/hooks/<harness>-warnings.log`.
- **Add remote `record` transport coverage** explaining activation via `scryrs.json` remote section and `SCRYRS_REMOTE_*` env vars, config precedence, no-dual-write/no-local-fallback semantics, the divergent remote output envelope (`transport`, `duplicate`, `failed` fields), and failure modes.
- **Correct dashboard endpoint table** to include `GET /api/meta` and `GET /api/sessions/:sessionId` alongside the existing three endpoints, matching `server.rs` router and `dashboard.rs` help text.
- **Remove `hook` from the out-of-scope list** — it is now a first-class implemented command.
- **Rename nav label** from `CLI v0 Contract` to `CLI Reference` in `_nav.json` while preserving the file slug (`cli-v0-contract`) for permalink stability.
- **Preserve and update cross-links** to `hotspots.md` and `trace-hook-contract.md` for deeper domain detail.

## Impact

- **Affected files:** `.devagent/docs/docs/cli-v0-contract.md` (primary rewrite), `.devagent/docs/docs/_nav.json` (nav label), `.devagent/docs/docs/hotspots.md` and `trace-hook-contract.md` (cross-link verification only).
- **No CLI behavior changes:** this is a documentation-only change. No flags, output contracts, exit codes, endpoints, or command surfaces are modified.
- **No spec snapshot changes:** `cargo test -p scryrs-cli` snapshot tests are unaffected.
- **Downstream consumers:** agent integrators and feature developers relying on the CLI docs will see an accurate, discoverable reference that matches the executable surface.