## ADDED Requirements

### Requirement: Dashboard SPA loads hotspot report data

The dashboard SHALL fetch and display the hotspot report from `GET /api/hotspots` on load. The hotspot view SHALL be the landing page of the SPA.

#### Scenario: Landing page shows hotspot table

- **GIVEN** the hotspot API returns a non-empty `HotspotsReport`
- **WHEN** the SPA loads at the root path
- **THEN** it SHALL display a table of hotspot entries with columns: Rank, Subject, Score, Session Count, Total Events, First Seen, Last Seen
- **AND** the table SHALL be sortable by any column
- **AND** entries SHALL be ranked starting from 1

#### Scenario: Empty hotspot report shows empty state

- **GIVEN** the hotspot API returns a report with zero entries
- **WHEN** the SPA loads
- **THEN** it SHALL display an empty-state message indicating no hotspot data is available
- **AND** it SHALL NOT show an empty table

#### Scenario: Hotspot API error shows error state

- **GIVEN** the hotspot API returns a 404 or 502 error
- **WHEN** the SPA loads
- **THEN** it SHALL display an error message explaining the data source could not be read
- **AND** it SHALL provide a "Retry" button that re-fetches the data

### Requirement: Dashboard SPA provides per-subject drill-down

Clicking a hotspot entry SHALL navigate to a detail view for that subject, showing session history, event counts by type, and a timeline.

#### Scenario: Subject detail view shows event breakdown by type

- **GIVEN** a hotspot entry has evidence with multiple event types
- **WHEN** a user clicks the entry's subject name
- **THEN** a detail view SHALL display the subject name and kind
- **AND** a bar chart or table SHALL show counts per event type (FileOpened, SearchRun, SymbolInspected, CommandExecuted, DocRetrieved, EditMade, LookupErrored)
- **AND** the total event count and session count SHALL be displayed

#### Scenario: Subject detail view shows session timeline

- **GIVEN** a hotspot entry has events from multiple sessions
- **WHEN** a user views the subject detail
- **THEN** a timeline SHALL display each session as a horizontal bar with event markers
- **AND** each session SHALL show its start time and source

### Requirement: Dashboard SPA provides session view

The SPA SHALL have a dedicated sessions view that lists recent sessions and allows navigating to a session detail view.

#### Scenario: Sessions view lists recent sessions

- **GIVEN** the sessions API returns session data
- **WHEN** a user navigates to `/sessions`
- **THEN** a list of sessions SHALL be displayed, ordered by start time (most recent first)
- **AND** each session SHALL show: session ID (truncated), start time, end time (or "Active"), event count, and source

#### Scenario: Session detail shows events within that session

- **GIVEN** a session ID
- **WHEN** a user clicks a session from the list
- **THEN** the SPA navigates to `/sessions/:sessionId`
- **AND** it SHALL display all events for that session in a scrollable list
- **AND** each event SHALL show: event type, timestamp, subject, and a truncated payload preview

### Requirement: Dashboard SPA provides event distribution view

The SPA SHALL have a dedicated events view showing aggregate event distribution across all captured traces.

#### Scenario: Event distribution shows pie or bar chart by event type

- **WHEN** a user navigates to `/events`
- **THEN** the SPA SHALL display a chart showing event counts grouped by `event_type`
- **AND** the chart SHALL use a meaningful color palette (one color per event type)
- **AND** each segment SHALL show the event type name and count

#### Scenario: Event distribution can be filtered by session

- **GIVEN** the events view is loaded
- **WHEN** a user selects a session from a dropdown filter
- **THEN** the chart SHALL update to show only events from that session
- **AND** a "Clear filter" option SHALL be available

### Requirement: SPA navigation uses Vue Router with distinct routes

The SPA SHALL use client-side routing with named routes for each view.

#### Scenario: Routes are defined for each view

- **WHEN** the SPA is loaded
- **THEN** the following routes SHALL be registered:
  - `/` — Hotspot report (landing page)
  - `/subjects/:subjectKind/:subject` — Subject detail view
  - `/sessions` — Session list view
  - `/sessions/:sessionId` — Session detail view
  - `/events` — Event distribution view
  - `/about` — Dashboard version and documentation

#### Scenario: Unknown route shows 404 page

- **WHEN** a user navigates to an unrecognized path
- **THEN** the SPA SHALL display a "Page not found" message
- **AND** it SHALL provide a link back to the landing page

### Requirement: SPA is built with Vue 3 and Vite

The frontend SHALL be implemented as a Vue 3 SPA using the Composition API, built with Vite, and embedded in the Rust binary at compile time.

#### Scenario: Production build outputs to frontend/dist/

- **WHEN** `npm run build` (or equivalent) is executed in `crates/scryrs-dashboard/frontend/`
- **THEN** the output SHALL be written to `crates/scryrs-dashboard/frontend/dist/`
- **AND** the output SHALL include `index.html`, `assets/` directory with hashed JS and CSS files

#### Scenario: Build is embeddable in Rust binary

- **WHEN** `cargo build` compiles the `scryrs-dashboard` crate
- **THEN** the build script SHALL invoke the frontend build (`npm run build`) if the `dist/` directory is missing or stale
- **AND** the resulting `dist/` files SHALL be embedded using `rust-embed`

### Requirement: SPA architecture has extensible component slots for future phases

The component tree SHALL be structured so that graph and route views (Phase 4+) can be added as new route components without modifying existing dashboard components.

#### Scenario: New views require only new route registration

- **GIVEN** a Phase 4 implementation adds a graph visualization
- **WHEN** a developer adds a new route component under `src/views/GraphView.vue`
- **THEN** they SHALL register the route in the router configuration
- **AND** they SHALL add a navigation link in the sidebar/nav component
- **AND** they SHALL NOT need to modify any existing view component for the new route to appear
- **AND** the API client module SHALL allow adding new endpoint calls without changing existing calls

#### Scenario: Shared components are reusable

- **WHEN** a new view needs a data table, chart, or filter control
- **THEN** the existing shared component library SHALL be used
- **AND** the shared component SHALL accept props for customization (columns, data source, color scheme)
