## 1. Feature boundary and command registration

- [ ] 1.1 Add a new non-default `curator-llm` feature to `crates/scryrs-cli/Cargo.toml`, wire in `scryrs-curator-llm` behind that feature, and include it in `full` but not `default`.
- [ ] 1.2 Register `scryrs proposals assist` under the existing plural review command group, with nested `draft` and `group` subcommands available only when `curator-llm` is enabled.
- [ ] 1.3 Ensure builds without `curator-llm` keep default deterministic CLI behavior unchanged and omit the assist surface from feature-aware help and help-json output.

## 2. Draft assist contract

- [ ] 2.1 Implement `scryrs proposals assist draft <PATH> <ID> --model <MODEL_ID>` as a review-only command that loads and validates a pending proposal from `.scryrs/proposals/{id}.json`.
- [ ] 2.2 Build the draft EvidencePack from the source proposal evidence by default, with explicit `--include-hotspots` and `--include-graph` flags to widen deterministic evidence scope.
- [ ] 2.3 Expose flags for request and pack bounds: `--max-input-chars`, `--max-hotspots`, `--max-graph-nodes`, `--max-proposals`, `--max-documents`, `--max-output-tokens`, and `--timeout-ms`.
- [ ] 2.4 Emit the validated replacement `ProposalDocument` JSON to stdout by default, and require `--write` before creating a new content-addressed `.scryrs/proposals/{newId}.json` draft.
- [ ] 2.5 Preserve the source proposal file unchanged and document that rewritten drafts are new pending proposals rather than in-place mutations.

## 3. Group assist contract

- [ ] 3.1 Implement `scryrs proposals assist group <PATH> --model <MODEL_ID>` as a review-only grouping command that loads bounded graph and hotspot evidence from deterministic repository artifacts.
- [ ] 3.2 Reuse the bounded `EvidencePack` and strict `scryrs-curator-llm::suggest_grouping(...)` validation path so returned candidates preserve exact `sourceNodeIds` and evidence citations.
- [ ] 3.3 Emit grouping proposal candidates to stdout by default, and require `--write` before persisting any new `.scryrs/proposals/{proposalId}.json` files.
- [ ] 3.4 Keep grouping output non-authoritative: no graph, route, accepted/rejected, docs, or memory mutation is allowed.

## 4. Failure handling and protected boundaries

- [ ] 4.1 Fail the entire assist run on malformed JSON, unknown citations, unknown source node IDs, empty evidence, content-shape mismatch, over-budget request/input/output, missing required deterministic inputs, or unavailable provider integration.
- [ ] 4.2 Ensure assist commands never emit partial success sets, never skip invalid candidates, and never leave partial inbox writes behind.
- [ ] 4.3 Reuse or extend protected-path verification so assist commands prove they do not create, modify, or delete `.scryrs/accepted/`, `.scryrs/rejected/`, `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/`, or source proposal inbox files.

## 5. Help, docs, and tests

- [ ] 5.1 Update `scryrs --help`, `scryrs proposals --help`, and feature-aware `scryrs --help-json` to describe the optional assist surface, required `--model`, stdout-vs-write behavior, evidence-scope flags, default EvidencePack budgets, and citation rules.
- [ ] 5.2 Update committed help/help-json snapshots and command-surface tests for both feature-enabled and feature-disabled builds.
- [ ] 5.3 Add tests for feature gating, EvidencePack budget flag visibility, source-proposal no-mutation, protected-path no-mutation, fail-whole-run invalid-output behavior, and no-partial-write behavior.
- [ ] 5.4 Update `.devagent/docs/docs/proposals.md`, `.devagent/docs/docs/cli-v0-contract.md`, and `.devagent/docs/docs/production-suite.md` to explain LLM assistance as optional proposal-review aid rather than graph, route, docs, or review authority.