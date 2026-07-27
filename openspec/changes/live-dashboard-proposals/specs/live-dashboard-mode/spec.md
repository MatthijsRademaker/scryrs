## MODIFIED Requirements

### Requirement: Live dashboard navigation and views are mode-aware

In live mode, the dashboard SHALL render live hotspot rankings, Signals, and Proposals when live proposal read APIs are available. Proposal review controls SHALL be shown only when authenticated write authorization is available. Local mode SHALL retain filesystem-backed proposal behavior.

#### Scenario: Live navigation includes Proposals when configured

- **GIVEN** `/api/meta` reports live mode
- **AND** live proposal reads are available
- **WHEN** navigation renders
- **THEN** a Proposals entry is visible

#### Scenario: Live proposal review respects authorization

- **GIVEN** the user opens a pending live proposal
- **AND** review-write authorization is unavailable
- **WHEN** the detail view renders
- **THEN** review controls are hidden or disabled with an explicit explanation
- **AND** no review request is sent
