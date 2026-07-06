# proposal-route-audit Specification

## Purpose
TBD - created by archiving change task-7f0b6e6e-7c4b-46a7-ac4d-70036c6c0529. Update Purpose after archive.
## Requirements
### Requirement: proposals.md accepted-only publish boundary is confirmed accurate

`proposals.md` SHALL be audited to confirm the accepted-only publish boundary is correctly documented.

#### Scenario: Accepted-only boundary matches shipped behavior

- **GIVEN** `crates/scryrs-cli/src/publish.rs` reads only `.scryrs/accepted/*.json`
- **WHEN** `proposals.md` is audited
- **THEN** it SHALL state that accepted review decisions in `.scryrs/accepted/` are the only publish inputs
- **AND** it SHALL state that proposal review does not auto-publish
- **AND** if the current text already states this, no edits SHALL be made

#### Scenario: Three-zone artifact layout matches shipped behavior

- **GIVEN** `crates/scryrs-cli/src/proposals.rs` writes only to `.scryrs/accepted/` or `.scryrs/rejected/` and never mutates `.scryrs/proposals/`
- **WHEN** `proposals.md` is audited
- **THEN** the three-zone layout (proposals, accepted, rejected) SHALL match shipped behavior
- **AND** if the current text already matches, no edits SHALL be made

#### Scenario: Review CLI surface matches shipped behavior

- **GIVEN** `crates/scryrs-cli/src/proposals.rs` implements `scryrs proposals list|accept|reject` with `--reviewer`, `--rationale`, `--decided-at` metadata
- **WHEN** `proposals.md` is audited
- **THEN** the documented review CLI surface SHALL match shipped behavior
- **AND** if the current text already matches, no edits SHALL be made

#### Scenario: debugging_playbook generation is correctly documented

- **GIVEN** `crates/scryrs-curator/src/lib.rs` generates `debugging_playbook` when FailedLookup count >= 2
- **WHEN** `proposals.md` is audited
- **THEN** `debugging_playbook` SHALL be listed as a generated target type with the FailedLookup >= 2 threshold
- **AND** if the current text already states this, no edits SHALL be made

### Requirement: route-manifests.md limitations match shipped explain behavior

`route-manifests.md` SHALL be audited to confirm that deterministic matching, packed relevance, loadTarget/non_loadable boundaries, and route bundle contract match shipped behavior.

#### Scenario: Deterministic matching is correctly documented

- **GIVEN** `crates/scryrs-runtime/src/lib.rs` ranks explain matches by `(tier DESC, score DESC, count DESC, manifest_index ASC, route_id ASC)`
- **WHEN** `route-manifests.md` is audited
- **THEN** the documented matching and ranking behavior SHALL match shipped behavior
- **AND** if the current text already matches, no edits SHALL be made

#### Scenario: Packed relevance is correctly documented as non-authoritative

- **GIVEN** `relevance` is a packed display derivative (`tier * 1_000_000_000 + min(score, 999_999) * 1_000 + min(count, 999)`), not the sort key
- **WHEN** `route-manifests.md` is audited
- **THEN** the documented relevance semantics SHALL match shipped behavior
- **AND** if the current text already matches, no edits SHALL be made

#### Scenario: loadTarget boundaries are correctly documented

- **GIVEN** `crates/scryrs-cli/src/route.rs` derives `loadTarget` as `file`, `doc_page`, or explicit `non_loadable` with syntax-only validation
- **WHEN** `route-manifests.md` is audited
- **THEN** the documented loadTarget behavior SHALL match shipped behavior
- **AND** it SHALL state that `non_loadable` targets carry no fake reference
- **AND** it SHALL state that there is no automatic source/docs loading
- **AND** if the current text already matches, no edits SHALL be made

#### Scenario: Route bundle contract is correctly documented

- **GIVEN** `crates/scryrs-cli/src/route_bundle.rs` ships `scryrs route bundle <PATH> --query <TEXT> --limit <N>`
- **WHEN** `route-manifests.md` is audited
- **THEN** the documented bundle behavior SHALL match shipped behavior
- **AND** if the current text already matches, no edits SHALL be made

