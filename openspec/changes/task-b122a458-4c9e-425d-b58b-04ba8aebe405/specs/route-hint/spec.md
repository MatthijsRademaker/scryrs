## MODIFIED Requirements

### Requirement: Route hint items carry structured identity, target, rank, and evidence

Each `RouteHintItem` SHALL include `routeId` (the source route entry id), `target` (the stable source route target string), `loadTarget` (optional structured load target copied from the source `RouteEntry`), `label` (human-readable label), `rank` (1-based ordinal from manifest sort order), `relevance` (optional; omitted by plain `hints_from_manifest` projection and populated for explain-derived query matches emitted by `scryrs route explain` and reused by `scryrs route bundle`), `reason` (deterministic template text, with load-target kind in the base template and explain-specific query-match suffix when applicable), and `evidence` (provenance links copied from the source `RouteEntry`).

#### Scenario: Plain projection omits relevance

- **GIVEN** any valid route manifest
- **WHEN** `hints_from_manifest` projects hints
- **THEN** every `RouteHintItem.relevance` is `None`
- **AND** the serialized JSON excludes the `relevance` field entirely

#### Scenario: Explain-derived matches populate deterministic relevance

- **GIVEN** a route entry matched by `scryrs route explain`
- **WHEN** the explain handler serializes the matched hint
- **THEN** `RouteHintItem.relevance` is present as a numeric `u32`
- **AND** the value equals `tier * 1_000_000_000 + min(total_evidence_score, 999_999) * 1_000 + min(evidence_count, 999)`

#### Scenario: Bundle targets reuse the explain-derived hint fields

- **GIVEN** a route entry included in `scryrs route bundle`
- **WHEN** the bundle serializes its `targets` entry
- **THEN** the target carries the same `routeId`, `target`, `loadTarget`, `label`, `rank`, `relevance`, `reason`, and `evidence` field shape as the underlying explain-derived hint
- **AND** the bundle does not invent a second per-target contract

### Requirement: Route-hint contract is documented for consumers

The CLI help surface (`--help` and `--help-json`), the CLI contract documentation (`cli-v0-contract.md`), and the route-manifest documentation (`route-manifests.md`) SHALL document the route-hint field shape, its evidence sources, optional `loadTarget`, `rank` as the manifest ordinal, plain-projection `relevance` omission, and explain-derived `relevance` population. Those surfaces SHALL also distinguish `scryrs route explain` as the unbounded diagnostic output and `scryrs route bundle` as a bounded plan that reuses the same per-target fields inside `targets`.

#### Scenario: Help text distinguishes plain hints, explain, and bundle

- **GIVEN** the route-hint contract is defined
- **WHEN** the user runs `scryrs --help`
- **THEN** the help output explains that plain route-hint projection omits `relevance`
- **AND** it explains that explain-derived outputs populate deterministic `relevance`
- **AND** it distinguishes `bundle` as the bounded planning surface and `explain` as the diagnostic surface

#### Scenario: Consumer docs distinguish rank, relevance, and bundle usage

- **GIVEN** the route-hint contract documentation exists
- **WHEN** a consumer reads `cli-v0-contract.md` or `route-manifests.md`
- **THEN** the documentation states that `rank` is the deterministic manifest ordinal
- **AND** it states that explain-derived `relevance` is a deterministic packed score for matched hints
- **AND** it states that `scryrs route bundle` reuses those per-target fields inside a bounded `targets` array
