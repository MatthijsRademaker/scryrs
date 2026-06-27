## Context

The project documentation at `.devagent/docs/docs/cli-v0-contract.md` serves agent integrators and developers as the canonical CLI reference. The current page has five structural defects identified during refinement:

1. **Stale command count:** Claims "five implemented commands" — the executable surface has six (`hotspots`, `record`, `hook`, `init`, `dashboard`, `server` per `dispatch.rs`).
2. **Missing `hook` section:** The `hook` command is a user-facing harness integration entry point with fail-open semantics (`hook.rs`), but the docs have no dedicated section and list it under "Out of scope."
3. **No remote `record` transport:** The implemented remote mode (`remote_config.rs`, `scryrs.json`) — with separate output envelope, config precedence, and no-dual-write semantics — is absent from the docs.
4. **Stale dashboard endpoints:** The dashboard REST API table lists 3 endpoints; the actual router (`server.rs`) serves 5 (`/api/meta`, `/api/hotspots`, `/api/sessions`, `/api/sessions/:sessionId`, `/api/events`).
5. **Architecture-first framing:** The page opens with technical contract language and never explains what problem scryrs CLI solves or which workflows it enables.

The refinement round produced unanimous consensus: rewrite the page as a single domain-driven reference, add a workflow-first opening, document all six commands, correct stale tables, and rename the nav label for discoverability while preserving the file slug.

## Goals / Non-Goals

### Goals
- Rewrite `cli-v0-contract.md` as a single file with a domain-driven opening that explains the user problem, the scryrs solution, and the two main workflow paths (local observe→detect, central live-ingest).
- Document exactly six implemented commands (`hotspots`, `record`, `hook`, `init`, `dashboard`, `server`) with dedicated sections matching `help_text.rs`, `help_json.rs`, and `dispatch.rs`.
- Add a dedicated `hook` section covering fail-open semantics (always exits 0, empty stdout), supported harnesses (`claude-code` via stdin, `pi` via `--file`), and the warning-log path at `.scryrs/hooks/<harness>-warnings.log`.
- Add remote `record` transport coverage explaining config precedence, no-dual-write/no-local-fallback semantics, the remote output envelope (`transport`, `duplicate`, `failed` fields), and failure modes.
- Correct dashboard endpoint table to include all five implemented routes (`/api/meta`, `/api/hotspots`, `/api/sessions`, `/api/sessions/:sessionId`, `/api/events`).
- Remove `hook` from the out-of-scope list and replace the explicit future-command list with a general "unknown commands exit 2" statement.
- Rename nav label from `CLI v0 Contract` to `CLI Reference` in `_nav.json` while preserving file slug `cli-v0-contract`.
- Cross-link to `hotspots.md` (domain concept depth) and `trace-hook-contract.md` (hook integration semantics).

### Non-Goals
- Do not change CLI behavior, flags, endpoints, or output contracts.
- Do not split the CLI page into multiple pages — keep as a single file.
- Do not rewrite architecture, roadmap, vision, or non-CLI product docs.
- Do not document future/speculative commands beyond marking them as out-of-scope.
- Do not change the file slug (`cli-v0-contract`) — only the nav display label.

## Decisions

1. **Single-file rewrite:** Keep `cli-v0-contract.md` as the single canonical CLI reference rather than splitting into separate workflow and contract pages. Rationale: Splitting creates maintenance overhead and discoverability burden for readers. A single well-structured page (domain intro → workflow picker → command reference → cross-links) gives readers both orientation and precision without breaking existing link targets.

2. **Nav label rename to "CLI Reference":** Update `_nav.json` display text from `CLI v0 Contract` to `CLI Reference` while keeping the slug `cli-v0-contract`. The slug preservation guarantees zero permalink breakage. The label change makes the page more discoverable for readers starting with user workflows.

3. **Remote record summarized with cross-link:** The CLI page summarizes remote transport mode (activation, output envelope differences, config precedence, failure semantics) with a link to `scryrs.json` and `trace-hook-contract.md` for exhaustive config schema detail. Cross-references prevent drift between docs.

4. **Hook gets both `scryrs hook` command section AND agent-facing contract subsection:** Following the existing pattern where `record`, `hotspots`, `dashboard`, and `server` each have a "When to call" subsection, `hook` gets the same treatment.

5. **Dashboard endpoints include all five routes but not per-endpoint response schemas:** The command reference table lists all five endpoints with descriptions; individual response structures belong in the REST API contract table already present in the page, not in a duplicative second table.

6. **Future commands list replaced with general statement:** The explicit list of eight future command names (`components`, `trace`, `propose`, etc.) is replaced by a general statement that any unknown command exits 2. This avoids maintenance burden when command names evolve.

## Risks

| Risk | Mitigation |
|------|-----------|
| CLI source could evolve during docs rewrite | Reference Rust source files directly as truth anchor, not memory or snapshots |
| Domain opening could drift into product positioning language that belongs in vision.md/roadmap.mdx | Cross-link to vision.md for the broader product loop and hotspots.md for domain concept depth rather than duplicating |
| Nav label rename could break external links | Preserve the file slug (`cli-v0-contract`) — only change display text in `_nav.json` |
| Remote ingest coverage could drift from `scryrs.json` if updated independently | Summarize in CLI page with explicit link to `scryrs.json` and `trace-hook-contract.md` for authoritative config schema |

## Traceability

- **Task:** `task:38ad5a05-c745-4bb5-835a-aa8153014618` (original backlog request)
- **Dossier:** `dossier:2026-06-27T09:20:24.024Z` (exploration evidence)
- **Decisions:** `decision:1-swarm-architect-recommendation`, `decision:1-swarm-lead-dev-recommendation`, `decision:1-swarm-reviewer-recommendation` (unanimous round-1 consensus)
- **Round outputs:** `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- **Source evidence:** `help_text.rs`, `help_json.rs`, `dispatch.rs`, `hook.rs`, `remote_config.rs`, `dashboard.rs`, `server.rs` (cli and dashboard crate), `scryrs.json`, `_nav.json`, `hotspots.md`, `vision.md`, `roadmap.mdx`