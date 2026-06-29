## Context

The production-hardening task is not a new feature lane. The refinement evidence converged on an operational gap: scryrs already ships the major parts of the observe -> detect -> organize -> review -> publish -> route loop, but maintainers and installed users do not have one diagnosis path for an installation and one authoritative release gate for the full suite. Existing scripts already cover formatting, tests, security scanning, install verification, hook capture, live hotspots, and docs publishing. Existing CLI code already exposes dispatch, help-json metadata, init-side filesystem effects, and local-vs-live mode resolution. The missing work is composition, explicit operator-facing contracts, and documentation.

## Goals / Non-Goals

### Goals

- Add `scryrs doctor` as a native CLI command with a documented public contract.
- Report binary version, feature availability, resolved mode, store status, hook configuration status, live server reachability when configured, and docs links.
- Add `scripts/verify-production-suite` as the single authoritative production gate.
- Add the missing deterministic core-artifact-loop verification and include it in the production gate.
- Add runnable privacy/security verification for telemetry defaults and document the remaining trace/privacy boundaries.
- Update production and verification docs so maintainers know exactly how to diagnose an install and run a release gate.

### Non-Goals

- Adding hosted multi-tenant behavior, auth, TLS, SaaS release infrastructure, or dashboard mutation flows.
- Redesigning trace schemas, hotspot scoring, graph or route contracts, proposal contracts, or adapter contracts.
- Introducing model-driven logic into ingest, scoring, graph truth, route truth, proposal validation, or release gating.
- Making hook failures block the harness; fail-open behavior remains intact.
- Claiming automated native macOS proof from the Linux Docker environment.
- Expanding this change into automated live dashboard browser E2E; dashboard verification remains a documented manual smoke boundary here.

## Decisions

### Decision 1: `scryrs doctor` is a native CLI command with dual output modes

Implement `scryrs doctor` in the shipped CLI, not as a repository-only script. The default output is human-readable for installed users. `--json` exposes the same diagnostic categories for automation. The command reports:

- binary version and shipped command surface / feature availability
- resolved mode using the same local-vs-live resolution logic as record transport
- local store status
- Claude Code and Pi hook configuration status where present
- live server reachability when live mode is configured
- relevant docs links

### Decision 2: Doctor uses a three-tier findings model and exit code 2 for structural failures

Each doctor check emits `ok`, `warn`, or `error`. The command exits `0` when findings are only `ok` or `warn`, and exits `2` when any structural `error` is present. Structural errors include unusable configured live mode, corrupt or unreadable configuration/store state, and other conditions that make the diagnosed setup non-operational. Advisory conditions such as an uninitialized local workspace or a missing optional hook remain warnings.

### Decision 3: Production verification is composition-first with one authoritative entrypoint

Create `scripts/verify-production-suite` as the only authoritative production-readiness command. It orchestrates existing lanes rather than duplicating them, prints clear lane headers, and exits non-zero on the first failed lane. The heavy lane is exposed through `scripts/precommit-run --production` for explicit invocation and is not promoted to the default PR gate in this change.

### Decision 4: The deterministic core artifact loop is a required new verification lane

No existing script proves the full local artifact loop. Add a dedicated deterministic verification path for `record -> hotspots -> graph -> route -> propose -> proposals accept`, executed through the real `scryrs` binary against fixture data. The verification asserts the expected deterministic artifacts exist in their documented locations and that proposal review acceptance produces accepted evidence.

### Decision 5: Privacy verification must use runnable assertions, not comments or source greps

The production suite includes a runnable privacy assertion lane that verifies compiled telemetry/privacy defaults programmatically. The suite documentation also maps the remaining privacy boundaries to their proving lanes: telemetry opt-in defaults, redaction defaults, debug-gated Bash capture, fail-open hook behavior, and remote-mode no-dual-write / no-local-fallback behavior.

### Decision 6: Live dashboard automation and native macOS automation stay out of scope for this round

The production gate covers the live server loop, not automated dashboard browser smoke. Verification docs must state that the live dashboard remains a documented manual smoke path. Linux install verification stays automated through the existing installer verification lane. macOS verification is documented as a manual maintainer lane with exact commands and an explicit statement that current Linux-only automation does not prove Darwin behavior.

## Risks

| Risk | Mitigation |
| --- | --- |
| `scryrs doctor` becomes an underspecified public surface | Define the command contract in the CLI reference, including required output categories, severity semantics, JSON mode, and exit policy. |
| The production gate becomes slow or hard to debug | Keep composition explicit, print per-lane headers, and preserve each sub-lane as an independently runnable script. |
| The new core-artifact-loop fixture becomes flaky | Use deterministic local fixture input and assert artifact presence and deterministic transitions only. |
| Privacy assertions drift into source-text checks | Require compiled test assertions for telemetry defaults and document other boundaries against existing verification lanes and contracts. |
| Maintainers misread dashboard or macOS coverage as automated | Document both boundaries explicitly in the verification README and Production Suite Plan. |

## Conflict Resolution

1. **Doctor surface vs script**: refinement considered a command or script, but accepted decisions converged on a native `scryrs doctor` command because installed-user diagnosis must work without a source checkout.
2. **Doctor output and exit policy**: open questions around human vs JSON output and structural failure handling are resolved as human-readable by default, `--json` for automation, `ok|warn|error` severities, and exit `2` for structural errors.
3. **Core artifact loop scope**: the original proposal sketch treated this lane as conditional; refinement resolved that it is mandatory because no inspected existing script covers the full deterministic loop.
4. **macOS verification**: refinement required a concrete strategy. The accepted synthesis uses exact documented manual verification rather than pretending Linux Docker proves macOS behavior.
5. **Live dashboard coverage**: the production gate verifies live server behavior only. Dashboard live mode remains a documented manual smoke boundary in this change.

## Traceability

| Source | How it is used |
| --- | --- |
| Task `782aa74d-a495-4a24-b877-ae014b85137b` | Defines the diagnostic path, production gate, privacy/security, packaging, and documentation acceptance criteria. |
| Dossier `2026-06-29T22:04:29.775Z` | Supplies the goals, non-goals, affected areas, assumptions, proposal sketch, and refined acceptance criteria. |
| Accepted decisions `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation` | Fix the doctor command shape, production-suite composition, mandatory core-artifact-loop lane, privacy assertions, dashboard boundary, and macOS manual verification posture. |
| Validated round outputs `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer` | Provide the contract details, blockers to resolve, and final scope boundaries that this design consolidates. |
| Current artifact snapshot `initial` | Confirms the change currently contains only placeholders and needs full proposal/design/spec/task publication. |