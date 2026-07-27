## MODIFIED Requirements

### Requirement: Subjects are grouped by subject_kind and subject

The system SHALL group events by the composite key `(subject_kind, subject)` for scoring and ranking. Two events with the same subject string but different `subject_kind` values SHALL be scored as independent entries.

Because path subjects are persisted in canonical repository-relative form (see the `trace-hook-contract` normalization requirement), a single file addressed by an agent in more than one way SHALL produce exactly one entry. Grouping SHALL remain a byte comparison on the stored subject — scoring SHALL NOT perform its own path normalization at read time, so that every consumer of the store agrees on subject identity.

Repository-external path subjects SHALL be grouped separately from repository-relative subjects.

#### Scenario: Same subject in different kinds produces separate entries

- **GIVEN** a `FileOpened` event with `subject_kind = "file"` and `subject = "routing"`
- **AND** a `SearchRun` event with `subject_kind = "search"` and `subject = "routing"`
- **WHEN** hotspots are scored
- **THEN** two separate entries are produced: one with `subjectKind = "file"` and one with `subjectKind = "search"`

#### Scenario: Same subject kind and subject are grouped together

- **GIVEN** two `FileOpened` events with `subject_kind = "file"` and `subject = "src/main.rs"`
- **AND** one `EditMade` event with `subject_kind = "file"` and `subject = "src/main.rs"`
- **WHEN** hotspots are scored
- **THEN** a single entry is produced with `subjectKind = "file"`, `subject = "src/main.rs"`
- **AND** `counts.eventType` includes both `"FileOpened": 2` and `"EditMade": 1`

#### Scenario: A file addressed absolutely and relatively produces one entry

- **GIVEN** events recorded after subject normalization for the same file addressed both absolutely and relatively
- **WHEN** hotspots are scored
- **THEN** a single entry is produced whose `count` reflects every event for that file
- **AND** no second entry exists for an alternate spelling of the same path

#### Scenario: Scoring does not normalize at read time

- **GIVEN** a store containing an un-normalized subject from before the normalization cut-line
- **WHEN** hotspots are scored
- **THEN** the subject is grouped as stored without read-time rewriting
- **AND** `scryrs hotspots` and the trace query surface report the same subject identity

#### Scenario: External paths group separately

- **GIVEN** an external path subject and a repository-relative subject sharing trailing path segments
- **WHEN** hotspots are scored
- **THEN** two separate entries are produced
