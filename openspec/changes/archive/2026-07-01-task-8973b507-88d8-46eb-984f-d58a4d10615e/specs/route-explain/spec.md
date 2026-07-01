## MODIFIED Requirements

### Requirement: Matching is tiered with documented tie-break

Matched entries SHALL be ordered deterministically by the full sort tuple `(best_match_tier DESC, total_evidence_score DESC, evidence_count DESC, manifest_index ASC, route_id ASC)`. `best_match_tier` remains exact (tier 3) before prefix (tier 2) before substring (tier 1), determined across `label`, `subject`, `id`, `target`, `kind`, and `evidence_links[].subject`. `total_evidence_score` SHALL be the saturating sum of `EvidenceLink.score.unwrap_or(0)` across the route's evidence links, with absent scores contributing zero. `evidence_count` SHALL be `RouteEntry.evidenceLinks.len()`. The full sort tuple SHALL be authoritative for ordering; the serialized `relevance` value SHALL NOT be used as the sort key. Only entries that match at least one field SHALL appear in the output.

#### Scenario: Exact match outranks higher-evidence prefix match

- **GIVEN** entry A matches the query exactly
- **AND** entry B matches the same query by prefix only but has a higher evidence score
- **WHEN** `scryrs route explain <PATH> --query <TEXT>` runs
- **THEN** entry A appears before entry B
- **AND** match tier outranks evidence score

#### Scenario: Higher evidence score wins within the same tier

- **GIVEN** two entries match within the same best-match tier
- **AND** entry A has a higher `total_evidence_score` than entry B
- **WHEN** the explain handler sorts matches
- **THEN** entry A appears before entry B

#### Scenario: Higher evidence count wins when tier and score tie

- **GIVEN** two entries match within the same best-match tier
- **AND** both entries have the same `total_evidence_score`
- **AND** entry A has more evidence links than entry B
- **WHEN** the explain handler sorts matches
- **THEN** entry A appears before entry B

#### Scenario: Stable tie-breaks preserve deterministic order

- **GIVEN** two entries have the same best-match tier, `total_evidence_score`, and `evidence_count`
- **WHEN** the explain handler sorts matches
- **THEN** the entry earlier in manifest order appears first
- **AND** `route_id` remains the final defensive deterministic tie-break

#### Scenario: Best match tier across fields determines ranking

- **GIVEN** entry A matches `id` exactly and `label` by substring
- **AND** entry B matches `subject` by prefix only
- **WHEN** the explain handler computes match tiers
- **THEN** entry A is tier 3 and entry B is tier 2
- **AND** entry A appears before entry B

### Requirement: Output is a valid RouteHintDocument with extended reason

The explain handler SHALL emit a `RouteHintDocument` with `schemaVersion` equal to `HINT_SCHEMA_VERSION`. Each matched `RouteHintItem` SHALL populate `relevance` with a deterministic packed `u32` value computed as `tier * 1_000_000_000 + min(total_evidence_score, 999_999) * 1_000 + min(evidence_count, 999)`. The packed value SHALL be documented as a display-friendly monotonic derivative of the authoritative sort tuple, not the ordering primitive. Each `RouteHintItem.reason` SHALL append query-match provenance in the format `; query match on {fields}` where `{fields}` is a comma-separated, alphabetically sorted list of matched field names. Unmatched entries SHALL be excluded entirely.

#### Scenario: Output schema matches RouteHintDocument contract

- **GIVEN** a valid route manifest with matching entries
- **WHEN** `scryrs route explain <PATH> --query "auth"` runs
- **THEN** the stdout output is valid single-line JSON
- **AND** it deserializes as `RouteHintDocument` with `schemaVersion = HINT_SCHEMA_VERSION`
- **AND** every returned `RouteHintItem` has `routeId`, `target`, `label`, `rank`, `relevance`, `reason`, and `evidence` fields

#### Scenario: Relevance is populated from match tier and evidence

- **GIVEN** a matched entry with best-match tier `2`, `total_evidence_score = 15`, and `evidence_count = 3`
- **WHEN** the explain handler serializes the hint
- **THEN** `relevance` equals `2000015003`

#### Scenario: Reason cites query-match provenance

- **GIVEN** a route entry with `label = "auth"` and `subject = "authentication"` that matches query `auth` on both fields
- **WHEN** the explain handler generates the reason
- **THEN** `reason` contains `"; query match on label, subject"`
- **AND** the base template is preserved before the match suffix

#### Scenario: Reason for evidence-only match

- **GIVEN** a route entry where only `evidence_links[].subject` matches the query
- **WHEN** the explain handler generates the reason
- **THEN** `reason` contains `"; query match on evidence.subject"`
- **AND** no other field names are listed

### Requirement: Explain is documented for consumers

The CLI help (`--help`), CLI machine-readable surface (`--help-json`), route-manifest documentation (`route-manifests.md`), and CLI contract documentation (`cli-v0-contract.md`) SHALL document the explain command, its arguments, output format, match fields, deterministic tie-break chain, packed relevance formula, and zero-match contract. Those surfaces SHALL distinguish `rank` as the manifest ordinal from explain `relevance` as the deterministic packed score, and SHALL state that plain `hints_from_manifest` output still omits `relevance`.

#### Scenario: Help text includes ranking chain and formula

- **GIVEN** the explain command is implemented
- **WHEN** the user runs `scryrs --help`
- **THEN** the help output documents exact > prefix > substring matching
- **AND** it documents evidence score, evidence count, and stable tie-breaks
- **AND** it documents how explain `relevance` is derived

#### Scenario: Help-json distinguishes plain projection from explain matches

- **GIVEN** the route explain contract is implemented
- **WHEN** the user runs `scryrs --help-json`
- **THEN** the JSON output describes the full explain ranking chain
- **AND** the route-hint field description states that plain projection omits `relevance`
- **AND** the explain command description states that query matches populate it with the packed deterministic score
- **AND** it states that the packed score is not the authoritative sort key

#### Scenario: Consumer docs no longer describe explain ranking as ordinal only

- **GIVEN** the explain command is implemented
- **WHEN** a consumer reads `route-manifests.md` or `cli-v0-contract.md`
- **THEN** the documentation describes explain ordering by match tier, evidence score, evidence count, and stable tie-breaks
- **AND** it does not describe explain `relevance` as always deferred or absent
