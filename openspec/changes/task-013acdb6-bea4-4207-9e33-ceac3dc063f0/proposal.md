## Why

`scryrs-curator-llm` already provides bounded, fail-loud library APIs for model-assisted drafting and semantic grouping, and `scryrs proposals list|accept|reject` already defines the review loop for pending proposals. What is still missing is the reviewer-facing UX contract that lets a reviewer ask for opt-in model help inside that review workflow without turning model output into graph, route, docs, or memory truth.

This change defines that contract after the review loop lands. It keeps model assistance nested under the plural `proposals` review surface, makes EvidencePack bounds and citation rules visible to users, preserves content-addressed proposal identity, and keeps default deterministic paths and builds model-free.

## What Changes

1. Add an opt-in `scryrs proposals assist` review subcommand group with `draft` and `group` operations, gated behind a new non-default `curator-llm` CLI feature.
2. Define `draft` as review-only model assistance for an existing pending proposal: it requires `--model`, emits a validated replacement `ProposalDocument` JSON to stdout by default, and only writes a new content-addressed inbox proposal when `--write` is explicitly provided.
3. Define `group` as review-only model assistance for semantic grouping: it loads bounded graph and hotspot evidence, emits validated `semantic_graph_grouping` proposal candidates to stdout by default, and only writes new pending proposal files when `--write` is explicitly provided.
4. Resolve the UX contract gaps called out during refinement:
   - draft output defaults to stdout because rewritten proposals get new content-addressed IDs;
   - draft evidence defaults to the source proposal evidence, with explicit `--include-hotspots` and `--include-graph` flags for broader deterministic evidence;
   - non-secret request and EvidencePack bounds are configured through flags only, not config files, env vars, or provider setup.
5. Surface the bounded EvidencePack contract in `--help`, feature-aware `--help-json`, and docs, including default caps (`max_input_chars=32000`, `max_hotspots=50`, `max_graph_nodes=100`, `max_proposals=20`, `max_documents=20`) and the rule that returned evidence must resolve to cited input-pack entries.
6. Preserve fail-whole-run behavior: malformed model output, unknown citations, unknown source node IDs, empty evidence, content-shape mismatch, over-budget input/output, or provider-unavailable conditions fail loudly with no partial writes and no protected-path mutation.
7. Update proposal, CLI, and production docs to explain LLM assistance as optional reviewer aid only, never as graph truth, route truth, published-doc truth, or review authority.

## Impact

- **CLI surface**: `crates/scryrs-cli` gains a feature-gated `proposals assist draft|group` contract under the existing plural review command group; `scryrs propose` remains deterministic and unchanged.
- **Feature boundary**: `crates/scryrs-cli/Cargo.toml` gains a new non-default `curator-llm` feature that can pull in `scryrs-curator-llm` without changing default-feature builds; `full` includes the new feature.
- **Model boundary**: `crates/scryrs-curator-llm` remains the model-aware implementation layer, while provider credential UX and hosted-provider wiring stay deferred.
- **Artifacts**: assist commands may emit stdout-only proposal candidates or, with explicit `--write`, create new `.scryrs/proposals/{proposalId}.json` files; they never mutate source proposals, accepted/rejected decisions, graph, routes, or docs.
- **Verification**: help/help-json, feature-gate, EvidencePack-budget visibility, fail-whole-run, no-partial-write, and protected-boundary tests are required for the new contract.