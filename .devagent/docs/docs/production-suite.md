# Production Suite Plan

scryrs is no longer a trace-capture scaffold. The production target is a closed evidence loop: observe agent work, score repeated attention, share live signals, organize evidence into graph and route artifacts, review proposed knowledge, publish approved knowledge, and let future agents load better context with explainable reasons.

## Operator entrypoints

Production hardening now has two explicit operator entrypoints:

- `scryrs doctor` — the installed-user diagnosis path for binary version, shipped command surface, resolved local-vs-live mode, local store status, harness hook status, live server reachability when configured, and docs links.
- `scripts/verify-production-suite` — the authoritative headless production-readiness gate for maintainers.

The heavy verification path is also exposed as `scripts/precommit-run --production`, but that wrapper does **not** replace the default lighter `scripts/precommit-run` posture in this change.

## Current State

| Area | State | Production gap |
| --- | --- | --- |
| Trace capture | `scryrs record`, `scryrs hook`, `scryrs init`, local SQLite, fail-open hooks, Pi and Claude Code adapters | Broader harness matrix and release-grade install diagnostics |
| Hotspots | Deterministic batch scoring and `.scryrs/hotspots.json` shipped | Long-running UX around trend/history still missing |
| Live hotspots | Central ingest server, idempotency, accumulators, query API, SSE signals shipped | Dashboard live mode and multi-agent end-to-end product workflow missing |
| Graph | `scryrs graph <PATH>` builds structural graph from hotspots plus docs navigation | Cross-domain edges and accepted evidence ingestion missing |
| Route manifests | `scryrs route <PATH>` emits `.scryrs/routes.json` from graph nodes | Runtime explanation and context loading decisions missing |
| Proposals | `ProposalDocument`, inbox layout, deterministic `scryrs propose <PATH>`, `scryrs proposals list|accept|reject`, accepted/rejected review artifacts, safety checks shipped | Dashboard review UX and broader accepted-evidence consumers still missing |
| Adapters | Shipped `scryrs publish markdown` and `scryrs publish rspress` commands delegate to the publishing adapters over reviewed `.scryrs/accepted/*.json` | Broader docs-surface targets beyond Markdown/Rspress are still missing |
| LLM assist | Bounded `scryrs-curator-llm` library shipped | Product integration must wait for acceptance lifecycle |

## Production loop

```text
Observe
  hooks / record / server ingest
    ↓
Detect
  batch hotspots + live accumulators + signals
    ↓
Organize
  graph build + route manifests
    ↓
Review
  proposal inbox + accept/reject ledger
    ↓
Publish
  generic Markdown / then Rspress / llms surfaces
    ↓
Route
  explainable route hints for future agents
```

The loop is production-ready only when every arrow is explicit, deterministic where it affects truth, and verified end to end.

## Authoritative production suite

`scripts/verify-production-suite` is the single headless release-verification command. It runs these required lanes and exits non-zero on the first failure:

1. `scripts/check`
2. `scripts/test --full`
3. `scripts/security`
4. `scripts/verify-install`
5. `scripts/verify-trace-capture`
6. `scripts/verify-live-hotspots`
7. `scripts/verify-core-artifact-loop`
8. `scripts/verify-privacy-defaults`
9. `scripts/verify-docs-publish`

### What each lane proves

| Lane | Proof |
| --- | --- |
| `scripts/check` | formatting, frontend check/build, workspace `cargo check`, clippy, docs publish verification |
| `scripts/test --full` | compiled Rust tests plus the lighter Claude-only `scripts/hook-test` wrapper lane |
| `scripts/security` | dependency policy and advisory audit |
| `scripts/verify-install` | Linux installer path and installed binary smoke |
| `scripts/verify-trace-capture` | real Claude Code + Pi hook capture through the shipped binary |
| `scripts/verify-live-hotspots` | remote ingest, idempotent replay, hotspot query, SSE replay/resume |
| `scripts/verify-core-artifact-loop` | deterministic local artifact loop `record -> hotspots -> graph -> route -> propose -> proposals accept` |
| `scripts/verify-privacy-defaults` | compiled telemetry/privacy defaults |
| `scripts/verify-docs-publish` | real `scryrs publish markdown` and `scryrs publish rspress` commands publish accepted knowledge through to built docs surfaces |

### Failure interpretation

The suite prints a lane header before each command. When the suite fails, the last printed lane header is the proving path that broke. Debug the failing lane directly rather than rerunning the full suite blindly.

## `scryrs doctor` contract in the production path

`scryrs doctor` is the public diagnosis command that complements the production suite. Its contract is intentionally narrower than release verification:

- default output is human-readable
- `--json` exposes the same categories for automation
- findings use `ok`, `warn`, and `error`
- exit `0` means only `ok`/`warn` findings were emitted
- exit `2` means at least one structural `error` was found

Structural errors include malformed `scryrs.json`, unreadable or unsupported local store state, incomplete live-mode identity, and unreachable configured live server. Advisory conditions such as an uninitialized local store or missing optional hook remain warnings.

## Critical boundaries

### Deterministic truth path

These paths must stay model-free:

- trace ingest
- hotspot scoring
- live accumulator updates
- graph materialization
- route-manifest materialization
- proposal validation
- accepted-evidence ingestion

### Review-first promotion path

Proposal files are not truth. Accepted evidence is truth candidate. Production verification keeps the boundary explicit:

```text
.scryrs/proposals/*.json       review inbox only — immutable proposal documents
.scryrs/accepted/*.json        reviewed evidence — accepted decisions with reviewed content
.scryrs/rejected/*.json        explicit rejection records — rejected decisions
.scryrs/graph.json             deterministic output from trace + docs + accepted evidence
.scryrs/routes.json            deterministic projection from graph
```

**Docs subtree ownership:** `.devagent/docs/docs/accepted-knowledge/` is owned exclusively by the `scryrs publish rspress` path, implemented by `scryrs-adapter-rspress`. It is cleared and regenerated on every publish run.

### Privacy proving boundary

The suite now has one explicit runnable privacy lane (`scripts/verify-privacy-defaults`) for compiled telemetry/privacy defaults. The remaining privacy boundaries are proved elsewhere and must stay documented as such:

| Boundary | Proving path |
| --- | --- |
| Telemetry opt-in default | `scripts/verify-privacy-defaults` |
| Prompt/source/path redaction defaults | `scripts/verify-privacy-defaults` |
| Remote prompt-storage default off | `scripts/verify-privacy-defaults` |
| Debug-gated Bash capture | `scripts/test --full`, `scripts/verify-trace-capture` |
| Fail-open hooks | `scripts/verify-trace-capture` |
| Remote no-dual-write / no-local-fallback | `scripts/test --full`, `scripts/verify-live-hotspots` |
| Dependency policy | `scripts/security` |

## Live dashboard and macOS scope boundaries

Two boundaries remain explicit in this change:

- **Live dashboard browser automation stays out of the automated production gate.** The production suite proves the live server contract only. Browser/dashboard smoke remains manual after `scripts/verify-live-hotspots` passes.
- **Linux automation does not prove native macOS behavior.** `scripts/verify-install` remains Linux/Docker automation. Real Darwin confidence still requires manual maintainer commands on macOS.

The exact manual commands and the explicit Linux-vs-macOS limitation are documented in `scripts/verification/README.md`.

## Milestones

| Milestone | Outcome | Must ship |
| --- | --- | --- |
| P1. Acceptance ledger | Proposals can become reviewed evidence without silent mutation | Accepted/rejected artifact contract, CLI review commands, validation, no-write guarantees |
| P2. Accepted evidence graph | Reviewed groupings and docs notes influence graph deterministically | Graph consumes accepted evidence; route manifests update from graph; provenance preserved |
| P3. Live dashboard | Multi-agent hotspots become visible product, not just API | Dashboard live mode, server API client, signal timeline, reconnect behavior |
| P4. Runtime explain | Agents can ask what to read and why | Route hint schema, `scryrs route explain`, deterministic evidence-backed reasons |
| P5. Publishing adapters | Reviewed knowledge leaves `.scryrs/` through explicit shipped CLI commands | `scryrs publish markdown` delegates to `scryrs-adapter-markdown` for generic Markdown output. `scryrs publish rspress` delegates to `scryrs-adapter-rspress` to write pages with Rspress frontmatter into `.devagent/docs/docs/accepted-knowledge/` and update `_nav.json`. `scripts/verify-docs-publish` proves both real CLI publish modes before checking `doc_build/llms.txt`. |
| P6. LLM assist UX | Models improve proposal quality without owning truth | Opt-in draft/group commands or UI action, bounded EvidencePack, citation validation, no auto-accept |
| P7. Production hardening | Suite can ship reliably | Release packaging, `scryrs doctor`, authoritative production suite, CI matrix, E2E live workflow, security and privacy checks |

## Related pages

- [CLI Reference](./cli-v0-contract.md) — public command contract, including `scryrs doctor`
- [Trace Hook Contract](./trace-hook-contract.md) — harness integration guarantees and fail-open rules
- [Architecture](./architecture.mdx) — crate boundaries and deterministic/model separation
- `scripts/verification/README.md` — lane-by-lane verification details, manual dashboard smoke, and macOS commands
