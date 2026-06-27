## Context

The scryrs project already ships:
- A deterministic `scryrs propose <PATH>` CLI command that loads `.scryrs/hotspots.json` and `.scryrs/graph.json`, delegates to `scryrs-curator::generate_proposals()`, and writes validated proposal files under `.scryrs/proposals/`.
- A canonical spec at `openspec/specs/proposal-generation/spec.md` whose "Source-of-truth artifacts are never mutated" requirement (lines 181-203) mandates that proposal generation SHALL NOT create or modify `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/`, or any source-of-truth ADR, skill, or memory files.
- A canonical spec at `openspec/specs/proposal-contract/spec.md` whose "Proposal inbox layout is deterministic and non-authoritative" requirement (lines 110-114) states proposal artifacts SHALL NOT directly mutate published docs, memory truth, `.scryrs/graph.json`, or `.scryrs/routes.json`.
- An existing integration test `source_of_truth_not_mutated()` at `crates/scryrs-cli/src/propose.rs:389-423` that seeds `.scryrs/graph.json` and `.scryrs/hotspots.json`, runs `write_proposals`, then verifies those two files are byte-for-byte unchanged.

What is missing is **coverage of the full protected set**: the test does not seed or assert `.scryrs/routes.json` or `.devagent/docs/`, and it does not enforce that all writes are confined to `.scryrs/proposals/`.

The archived implementation tasks (`openspec/changes/archive/2026-06-27-task-597db7ef-f2da-4d5e-9459-7c19601a5e85/tasks.md`) incorrectly claim at task 4.6 that the test covers `.scryrs/routes.json` and `.devagent/docs/`. This change corrects that gap.

## Goals / Non-Goals

### Goals
- Extend `source_of_truth_not_mutated()` to seed and byte-for-byte verify `.scryrs/routes.json` and `.devagent/docs/` files alongside the existing `.scryrs/graph.json` check.
- Add a whole-repo file-inventory diff (file paths + hashes before/after `write_proposals`) asserting that new or modified files are confined to `.scryrs/proposals/**`.
- Remove or relegate `.scryrs/hotspots.json` from the non-mutation assertion, aligning the test with the spec's protected-source-of-truth scope.
- Implement a dynamic protected-paths acceptlist so future source-of-truth destinations (e.g., adapter-managed ADR/skill/memory outputs) can be added without rewriting the test harness.
- Keep all changes in `#[cfg(test)]` within `crates/scryrs-cli/src/propose.rs`, following the established `tempfile::TempDir` pattern.

### Non-Goals
- Do not change proposal heuristics, target types, or inbox semantics.
- Do not add review UI, publishing adapters, or automatic acceptance/publishing behavior.
- Do not edit canonical/archived OpenSpec artifacts as part of this task.
- Do not change `write_proposals` production code unless the strengthened test reveals a real write-path bug.
- Do not introduce new crate dependencies.

## Decisions

### Decision 1: Verification strategy — byte-for-byte + full file-inventory diff
**Choice**: Use per-file byte-equality for the protected set (`.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/*`) combined with a whole-repo file-inventory snapshot that records all files with SHA-256 hashes before and after `write_proposals`. The inventory diff asserts that any added or modified files are under `.scryrs/proposals/**`.
**Rationale**: Byte-for-byte comparison is the strongest non-mutation proof for known protected files. The inventory diff catches any unexpected write outside `.scryrs/proposals/`, including to files the test didn't anticipate (e.g., a future source-of-truth path or a TempDir housekeeping file). SHA-256 hashes avoid false positives from mtime-only changes. The combination satisfies both the dossier acceptance criteria and the architect's recommendation for precise, extensible verification.
**Traceability**: architect round 1 (byte-for-byte + directory-snapshot strategy), lead-dev round 1 (whole-repo file-inventory diff), reviewer round 1 (SHA-256 hashing).

### Decision 2: Protected-path scope — graph.json, routes.json, .devagent/docs/ (not hotspots.json)
**Choice**: The dynamic protected-paths list SHALL be seeded with `.scryrs/graph.json`, `.scryrs/routes.json`, and `.devagent/docs/` (recursive). `.scryrs/hotspots.json` SHALL NOT be in the protected set because it is an input artifact read by `write_proposals`, not a source-of-truth output the spec requires to stay unmodified.
**Rationale**: The proposal-generation spec:183 explicitly names graph.json, routes.json, and .devagent/docs/ as the protected artifacts. Hotspots.json is the seed input that drives generation — including it in the non-mutation test creates false completeness. The architect's dynamic-allowlist recommendation ensures future paths can be added without code changes.
**Traceability**: architect round 1 (dynamic allowlist), lead-dev round 1 (remove hotspots from assertion), reviewer round 1 (align with spec).

### Decision 3: Test fixture content — minimal valid content for routes and docs
**Choice**: Seed `.scryrs/routes.json` with `{}` (empty JSON object — a minimal valid routes document). Seed `.devagent/docs/docs/_nav.json` with `[]` (empty array) and `.devagent/docs/docs/vision.md` with representative markdown content. Use the existing `write_test_graph`/`write_test_hotspots` helpers for graph and hotspots.
**Rationale**: The routes.json content must be valid but minimal — the test only needs to prove it isn't mutated, not that write_proposals handles complex routes. The docs seed mirrors the path structure that `graph.rs:44-45` uses (`.devagent/docs/docs/` + `_nav.json`). Two files (nav + one page) provides enough coverage to catch accidental writes while keeping test setup simple.
**Traceability**: reviewer round 1 (minimal valid routes.json content `{}`), lead-dev round 1 (mirror graph.rs docs path structure), dossier acceptance criteria (representative .devagent/docs/ files).

### Decision 4: Dynamic allowlist — parameterized, not hardcoded
**Choice**: The verification helper SHALL accept a `&[&str]` or `&[PathBuf]` list of protected paths, rather than hardcoding the list inside the helper. The `source_of_truth_not_mutated()` test SHALL pass the canonical protected paths as a caller-provided list.
**Rationale**: The architect explicitly recommended a dynamic allowlist for future-proofing. If adapter-managed ADR, skill, or memory outputs become source-of-truth destinations, adding them requires only a one-line change at the call site, not a test harness rewrite.
**Traceability**: architect round 1 (dynamic list of protected file paths).

### Decision 5: Bug-contingent fix — surgical, not broad
**Choice**: If the strengthened test reveals that `write_proposals` writes outside `.scryrs/proposals/`, fix ONLY the offending write instruction in the production code. DO NOT redesign proposal heuristics, target types, or inbox semantics.
**Rationale**: The dossier non-goals explicitly forbid changing proposal heuristics or inbox semantics unless a real safety bug is exposed. Even then, the fix must be scoped to the specific offending write, not a broader refactor.
**Traceability**: dossier non-goals, architect round 1 (fix offending write instruction only).

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Inventory diff false positives from TempDir housekeeping files | Low | The diff should be scoped to files within the temp repo root, and `.scryrs/proposals/**` is explicitly allowed. Any file outside that path that appears after `write_proposals` is a real concern. If TempDir creates unexpected files, the test can explicitly allowlist them or use a before/after file set difference. |
| Path canonicalization mismatch between test and production code | Low | Both test and production code use `std::fs` paths resolved from the same TempDir root. No symlink handling needed for temp dirs. |
| Test fragility if `.devagent/docs/` structure changes | Low | The test is co-located in the same module as the production code. Any structural change to docs paths would be part of a change that updates this test. |
| Strengthened test reveals real write-path bug, expanding scope | Low-Medium | The dossier acceptance criteria explicitly account for this: "If no unsafe write exists, production proposal-generation behavior remains unchanged." If a bug exists, the fix is scoped to the offending write instruction per Decision 5. |

## Traceability

- Task: `9b0c8757-a49f-4337-8f49-1f324778eba6` (Curator Foundation 03 — Verify proposal flow stays review-first)
- Dossier: `2026-06-27T22:01:48.060Z`
- Round 1 decisions: architect (dynamic allowlist, byte-for-byte + directory-snapshot), lead-dev (full inventory diff, remove hotspots, seed routes/docs), reviewer (SHA-256 hashes, minimal valid routes/docs content)
- Artifact base: `openspec/changes/task-9b0c8757-a49f-4337-8f49-1f324778eba6@initial`
- Source specs: `proposal-generation` (source-of-truth non-mutation requirement), `proposal-contract` (non-authoritative proposal inbox)
- Source code: `crates/scryrs-cli/src/propose.rs` (existing tests and production code), `crates/scryrs-cli/src/graph.rs:44-45` (docs path reference)