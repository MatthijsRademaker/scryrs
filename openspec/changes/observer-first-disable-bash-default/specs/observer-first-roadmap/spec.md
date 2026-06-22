## ADDED Requirements

### Requirement: Roadmap documents observer-first product boundary and future rewrite direction

The roadmap SHALL describe scryrs' current product boundary as observer-first hotspot detection on stable native tool signals. The roadmap SHALL also include a thin future-work note describing RTK-style command rewriting as later-phase optional optimizer work that may consume scryrs evidence, without presenting rewrites as part of current product scope.

#### Scenario: Roadmap keeps current product story observer-first

- **WHEN** a reader reviews `.devagent/docs/docs/roadmap.mdx`
- **THEN** the current product boundary emphasizes non-interfering trace capture and hotspot detection
- **AND** it does not present command rewrites as current implemented behavior

#### Scenario: Roadmap mentions later RTK-style rewrite concept narrowly

- **WHEN** a reader reviews the future phases or accepted limitations
- **THEN** they see a brief note that RTK-style rewrite/optimizer behavior is possible future work
- **AND** the note states that such work would come after observer-first evidence collection is established
