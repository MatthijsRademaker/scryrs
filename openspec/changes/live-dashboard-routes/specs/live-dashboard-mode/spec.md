## MODIFIED Requirements

### Requirement: Live dashboard navigation and views are mode-aware

In live mode, the dashboard SHALL render live hotspot rankings, Signals, and Routes when the repository has a published route manifest. Routes SHALL show an actionable missing-publication state when no manifest exists. Sessions, Events, and Proposals remain governed by their separate live capabilities.

#### Scenario: Live navigation includes Routes

- **GIVEN** `/api/meta` returns `mode: "live"`
- **WHEN** the dashboard shell renders navigation
- **THEN** a Routes navigation entry is visible
- **AND** the entry does not imply local artifact input

#### Scenario: Live route direct navigation is readable

- **GIVEN** the dashboard is in live mode
- **WHEN** the user navigates to `/routes`
- **THEN** the route view renders search or explicit publication/error state
- **AND** it does not render an unconditional unavailable card
