## MODIFIED Requirements

### Requirement: Motion is spring-based and restrained

Interface motion SHALL be spring-based and restrained. Structural and chrome transitions (nav state, card entrance, generic feed items) SHALL complete in approximately 200–400ms with ease-out timing and SHALL NOT overshoot or bounce. Data-bearing motion (e.g. a live hotspot signal arriving) MAY use spring physics with a single soft settle that briefly overshoots before coming to rest, scaled by the underlying data, but SHALL NOT use cartoonish or oscillating bounce and SHALL return the surface to a calm, still resting state. A dedicated spring-physics animation library (`motion-v`) MAY be used to implement this motion in addition to `tw-animate-css` utilities and reka-ui transition primitives. All motion SHALL honor the user's reduced-motion preference through a single shared source of truth.

#### Scenario: Chrome transitions respect the timing and easing budget

- **WHEN** a structural or chrome element (nav state, card entrance, generic feed item) transitions
- **THEN** the transition completes within roughly 200–400ms with ease-out timing
- **AND** it does not overshoot or bounce

#### Scenario: Data-bearing motion uses a single soft settle

- **WHEN** a data-bearing element (such as an arriving live hotspot signal) animates
- **THEN** it MAY use spring physics with a single soft settle scaled by the underlying data
- **AND** it does not use cartoonish or oscillating bounce
- **AND** the surface returns to a calm, still resting state once the motion completes

#### Scenario: Spring-physics library is permitted for data-bearing motion

- **WHEN** `crates/scryrs-dashboard/frontend/package.json` is inspected after the change
- **THEN** the spring-physics animation library `motion-v` MAY be present in addition to `tw-animate-css` and reka-ui
- **AND** no other unrelated runtime animation library is introduced
