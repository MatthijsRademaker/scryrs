## Context

scryrs already has working reference hooks, deterministic ingestion, and hotspot materialization. That foundation is good, but default Bash capture is weak evidence: command strings are unstable, compound commands stay unsplit, rewrite prefixes fragment subjects, and `CommandExecuted` has low scoring weight. Product boundary now needs to get stricter: observer-first, hotspot-focused, non-interfering, and biased toward stable harness-native signals.

Current repository state also gives one useful lever: Pi already uses `SCRYRS_DEBUG` for opt-in trace-hook diagnostics. Reusing one existing debug control is simpler than adding a second flag just for Bash. Claude Code docs and hook behavior can align on same env var so both harnesses share one observer/debug contract.

## Goals / Non-Goals

**Goals:**

- Make Bash trace capture opt-in instead of default across both reference hooks.
- Re-center default observed surface on stable native tools: reads, edits, search/navigation, document fetch.
- Keep `CommandExecuted` schema and scoring available for explicit debug sessions.
- Update verification, manifest metadata, and docs so default behavior and debug behavior are both explicit and testable.
- Add thin roadmap documentation for future RTK-style rewrite/optimizer direction without changing current execution semantics.

**Non-Goals:**

- No command rewriting, shell parsing, canonicalization, or optimizer runtime in this change.
- No change to Rust trace schema, hotspot scoring weights, or CLI output contracts.
- No attempt to split compound Bash commands or recover original-vs-rewritten intent.
- No expansion of hook scope into proxy execution, tool mutation, or business logic.

## Decisions

### 1. Gate Bash capture behind existing `SCRYRS_DEBUG`

Use non-empty `SCRYRS_DEBUG` as single cross-harness switch for Bash observation.

Rationale:

- Matches user intent: Bash becomes diagnostic-only.
- Reuses existing Pi debug contract instead of inventing another env var.
- Keeps rule simple: if debug is on, noisy command capture is acceptable.

Alternatives considered:

- **New `SCRYRS_TRACE_BASH` env var**: clearer single-purpose name, but adds config surface and duplicates debug intent.
- **Remove Bash entirely**: simplest observer story, but loses valuable explicit troubleshooting path.

### 2. Keep hook non-interference unchanged

Hooks still observe and forward only. They do not rewrite commands, alter results, proxy execution, or infer canonical commands.

Rationale:

- Preserves scryrs identity as observer first.
- Avoids mixing telemetry scope with optimizer scope before stable evidence model exists.

Alternative considered:

- **Thin runtime rewrite shim now**: rejected because it would shift scryrs from observer into execution middleware too early.

### 3. Preserve `CommandExecuted` schema and scoring, but downgrade default production importance

Do not remove `CommandExecutedPayload` or scoring rules. Simply reduce default emission frequency by gating Bash observation.

Rationale:

- Smallest behavioral change.
- Keeps backward-compatible trace family for debug sessions and future roadmap work.
- Avoids unnecessary churn in Rust crates and hotspot report contract.

Alternative considered:

- **Delete command event support entirely**: rejected because future rewrite/optimizer direction still needs command evidence in controlled sessions.

### 4. Represent Bash in manifest and docs as debug-only capability

Default intercepted-tool and event-family declarations should describe observer-first default behavior. Bash support should appear as debug-only metadata or limitation text, not default product surface.

Rationale:

- Keeps install-time and integration-time story honest.
- Prevents manifest from overselling noisy capture as core value.

Alternative considered:

- **Leave manifest unchanged and explain in prose only**: rejected because machine-readable metadata would contradict actual default behavior.

### 5. Verify both default suppression and debug opt-in paths

Cross-harness verification must assert two modes:

- default mode: no Bash trace event
- debug mode: Bash trace event still captured

Rationale:

- Prevents accidental regressions in either direction.
- Makes observer-first boundary executable truth instead of doc-only claim.

### 6. Roadmap rewrite concept stays documentation-only

Roadmap should add thin future direction describing RTK-style rewrite concept as later-phase optional optimizer work fed by observed evidence, not present feature.

Rationale:

- Captures strategic intent without polluting current product boundary.
- Keeps implementation scope surgical.

## Risks / Trade-offs

- **Reduced command evidence in normal sessions** → Mitigation: keep debug-gated Bash capture available via `SCRYRS_DEBUG`.
- **Potential confusion about `SCRYRS_DEBUG` doing both logging and Bash capture** → Mitigation: document one rule clearly in both hook READMEs, manifest limitations, and trace-hook contract.
- **Manifest/schema wording may become more conditional and less simple** → Mitigation: define default observed tools separately from debug-only Bash support.
- **Some existing verification and docs assume Bash always captured** → Mitigation: update both harness specs and fixtures together in same change.
- **Future optimizer work may want richer command metadata than current schema supports** → Mitigation: roadmap explicitly frames rewrite support as later work, not something current schema solves.

## Migration Plan

1. Change both reference hooks so Bash events are skipped unless `SCRYRS_DEBUG` is non-empty.
2. Update hook READMEs and trace-hook contract to describe observer-first default and debug-only Bash capture.
3. Update `scryrs.json` metadata to reflect default observed tools and debug-only Bash support.
4. Update verification fixtures to prove default suppression and debug-gated capture in both harnesses.
5. Add roadmap note describing future RTK-style rewrite concept as later-phase work.

Rollback:

- Restore Bash to default tracked-tool lists and remove debug gating in both hooks.
- Revert manifest/docs/verification changes together so product story remains consistent.

## Open Questions

- Should manifest expose debug-only Bash support in explicit fields (for example `debugOnlyInterceptedTools`) or as limitation prose attached to harness entries?
- Should roadmap place rewrite concept under Phase 7 runtime retrieval, or add narrower note under scope guardrails / future work section?
