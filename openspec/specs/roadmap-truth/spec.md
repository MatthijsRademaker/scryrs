# roadmap-truth Specification

## Purpose
TBD - created by archiving change task-7f0b6e6e-7c4b-46a7-ac4d-70036c6c0529. Update Purpose after archive.
## Requirements
### Requirement: Phase 6 narrative accurately states shipped and deferred surfaces

Phase 6 of `roadmap.mdx` SHALL explicitly mark shipped command surfaces and remaining product gaps. The phase SHALL NOT contain any claim that a shipped command surface is excluded or deferred.

#### Scenario: debugging_playbook is not claimed as excluded

- **GIVEN** `crates/scryrs-curator/src/lib.rs` generates `debugging_playbook` proposals when FailedLookup count >= 2
- **WHEN** `roadmap.mdx` Phase 6 narrative is read
- **THEN** it SHALL NOT contain the claim that "`debugging_playbook` remains excluded from v1 generation"
- **AND** it SHALL state that `debugging_playbook` proposals are generated when FailedLookup count >= 2

#### Scenario: Review CLI is listed as shipped

- **GIVEN** `crates/scryrs-cli/src/proposals.rs` implements `scryrs proposals list|accept|reject`
- **WHEN** `roadmap.mdx` Phase 6 narrative is read
- **THEN** it SHALL name the shipped `scryrs proposals list|accept|reject` review CLI surface

#### Scenario: Partial delivery marker exists with remaining gaps

- **GIVEN** Phase 6 has shipped `scryrs propose`, `scryrs proposals list|accept|reject`, and `debugging_playbook` generation
- **WHEN** `roadmap.mdx` Phase 6 narrative is read
- **THEN** it SHALL include a Partial delivery note naming the shipped surfaces
- **AND** it SHALL list remaining gaps explicitly (e.g., no dashboard review UX, no broader accepted-evidence consumers)

### Requirement: Phase 7 narrative includes partial delivery marker for publishing adapters

Phase 7 of `roadmap.mdx` SHALL include a partial-delivery note naming the shipped `scryrs publish markdown` and `scryrs publish rspress` commands.

#### Scenario: Shipped publish commands are named

- **GIVEN** `crates/scryrs-cli/src/publish.rs` exposes `scryrs publish markdown` and `scryrs publish rspress`
- **WHEN** `roadmap.mdx` Phase 7 narrative is read
- **THEN** it SHALL name both shipped publish commands
- **AND** it SHALL state the remaining gap (broader docs-surface targets beyond Markdown/Rspress)

### Requirement: Phase 8 narrative states shipped surface and product-completeness gap

Phase 8 of `roadmap.mdx` SHALL NOT present runtime retrieval as entirely future work. It SHALL mark shipped command surfaces while explicitly stating the product is not product-complete.

#### Scenario: Shipped route commands are named

- **GIVEN** `crates/scryrs-cli/src/route_explain.rs` ships `scryrs route explain --query` and `crates/scryrs-cli/src/route_bundle.rs` ships `scryrs route bundle`
- **AND** `crates/scryrs-runtime/src/lib.rs` ships `explain_hints` with deterministic matching and `hints_from_manifest` projection
- **WHEN** `roadmap.mdx` Phase 8 narrative is read
- **THEN** it SHALL name the shipped `scryrs route explain`, `scryrs route bundle`, route hint schema, and explain_hints surfaces

#### Scenario: Product-completeness gap is explicit

- **GIVEN** route explain uses deterministic manifest matching with packed display relevance and no automatic context loading
- **WHEN** `roadmap.mdx` Phase 8 narrative is read
- **THEN** it SHALL explicitly state this is NOT product-complete
- **AND** it SHALL state that retrieval is deterministic manifest matching with explicit non_loadable targets and no automatic context loading
- **AND** it SHALL NOT claim or imply that runtime retrieval is semantically useful or product-complete

### Requirement: Production-Ready Suite Path table P4 reflects shipped route explain

Row P4 of the Production-Ready Suite Path table in `roadmap.mdx` SHALL reflect that `scryrs route explain` is shipped.

#### Scenario: P4 row does not list route explain as future blocking work

- **GIVEN** `scryrs route explain --query` is shipped
- **WHEN** the Production-Ready Suite Path P4 row is read
- **THEN** it SHALL NOT list "Define route hint contract and implement `scryrs route explain`" as blocking future work
- **AND** it SHALL describe the actual remaining gap (e.g., semantic ranking and automatic context loading remain deferred)

### Requirement: Near-term milestones mark shipped items as delivered

Near-term milestones M7, M9, and M10 in `roadmap.mdx` SHALL be marked as delivered (✅ Shipped) since the corresponding commands are shipped.

#### Scenario: M7 review loop beta is marked delivered

- **GIVEN** `scryrs proposals list|accept|reject` is shipped
- **WHEN** the near-term milestones table is read
- **THEN** milestone M7 (Review loop beta) SHALL be marked as delivered with ✅ Shipped

#### Scenario: M9 runtime explain beta is marked delivered

- **GIVEN** `scryrs route explain --query` is shipped
- **WHEN** the near-term milestones table is read
- **THEN** milestone M9 (Runtime explain beta) SHALL be marked as delivered with ✅ Shipped

#### Scenario: M10 adapter suite beta is marked delivered

- **GIVEN** `scryrs publish markdown` and `scryrs publish rspress` are shipped
- **WHEN** the near-term milestones table is read
- **THEN** milestone M10 (Adapter suite beta) SHALL be marked as delivered with ✅ Shipped

### Requirement: graph.md cross-reference is not edited in this change

The `graph.md` file SHALL NOT be edited as part of this docs-truth alignment change.

#### Scenario: graph.md Route → Runtime retrieval wording remains unchanged

- **GIVEN** the task scope is limited to roadmap, production-suite, proposals, and route-manifests
- **WHEN** docs-truth edits are applied
- **THEN** `graph.md` SHALL remain unchanged
- **AND** the "Route → Runtime retrieval (future)" sentence SHALL NOT be modified

