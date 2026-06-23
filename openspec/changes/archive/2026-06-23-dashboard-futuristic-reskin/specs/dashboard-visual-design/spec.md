## ADDED Requirements

### Requirement: Dark-first visual identity derived from the logo

The dashboard SHALL present a dark-first visual identity derived from the scryrs logo: a deep midnight-navy canvas, frosted-glass surfaces, a single electric cyan-blue accent, and a warm flame gradient reserved exclusively for hotspot heat. The dark theme SHALL be the default; a light theme SHALL be retained as a secondary fallback.

#### Scenario: Dark theme is the default on first load

- **WHEN** the dashboard is loaded without a stored theme preference
- **THEN** the dark theme is applied
- **AND** the document background renders the deep midnight-navy canvas, not a light canvas

#### Scenario: Theme tokens are defined in oklch

- **WHEN** `crates/scryrs-dashboard/frontend/src/app/styles.css` is inspected
- **THEN** it defines a dark-first token set including a midnight-navy `--background`, raised glass surface color(s), a near-white foreground, an electric cyan-blue `--accent`/`--primary`, and a flame gradient token
- **AND** it retains a light theme token set as a secondary variant

#### Scenario: Light theme remains available as a fallback

- **WHEN** the light theme is active
- **THEN** all views remain legible and structurally identical to the dark theme
- **AND** no element relies on a glow effect that is invisible or broken on the light canvas

### Requirement: Chrome stays calm and data glows

The dashboard SHALL reserve color emphasis, glow, and motion for data. Structural chrome — navigation, surface backgrounds, borders, and non-data labels — SHALL remain near-monochrome. The cyan accent glow SHALL be applied to interactive/active and data-bearing elements; the flame gradient SHALL be applied only to hotspot heat indicators.

#### Scenario: Chrome is near-monochrome

- **WHEN** the app shell and any view's structural elements are inspected
- **THEN** navigation, card surfaces, borders, and structural labels use the neutral midnight/glass/ink tokens
- **AND** they do not use the flame gradient
- **AND** the cyan accent on chrome is limited to active/interactive states (e.g. active nav item, focus ring, hover)

#### Scenario: Flame gradient is exclusive to hotspot heat

- **WHEN** the flame gradient token is used anywhere in the dashboard
- **THEN** it appears only on hotspot heat/score indicators
- **AND** it is not used for navigation, generic accents, or non-hotspot surfaces

### Requirement: Frosted-glass surfaces and depth

Card and panel surfaces SHALL use a frosted-glass treatment: a translucent fill over the layered background with a backdrop blur and a hairline border. The dashboard SHALL convey depth through layered surfaces and at most one ambient aurora glow echoing the logo's lens, used sparingly behind a hero region.

#### Scenario: Cards render as frosted glass

- **WHEN** a card or panel surface is inspected in the dark theme
- **THEN** it renders a translucent fill with a backdrop blur and a hairline border
- **AND** it is visually distinct in elevation from the canvas behind it

#### Scenario: Ambient aurora is used at most once per view

- **WHEN** a view renders an ambient aurora/lens glow background effect
- **THEN** at most one such ambient glow is present in that view
- **AND** it sits behind content without reducing text legibility

### Requirement: Motion is spring-based and restrained

Interface motion SHALL be spring-based and restrained: transitions complete in approximately 200–400ms with ease-out timing and SHALL NOT use bouncy or cartoonish easing. Motion SHALL be implemented with the already-available `tw-animate-css` utilities and reka-ui transition primitives without adding new animation dependencies.

#### Scenario: Transitions respect the timing and easing budget

- **WHEN** an animated element (nav state, card entrance, event feed item) transitions
- **THEN** the transition completes within roughly 200–400ms with ease-out timing
- **AND** it does not overshoot or bounce

#### Scenario: No new animation dependency is introduced

- **WHEN** `crates/scryrs-dashboard/frontend/package.json` is inspected after the change
- **THEN** no new runtime animation library is added beyond `tw-animate-css` and reka-ui

### Requirement: App shell presents the scryrs brand lockup

The app shell SHALL present a glass navigation rail with a scryrs logo lockup, an active-item cyan edge-glow, and the midnight aurora canvas. The shell SHALL include a slot for a user-provided logo asset and SHALL degrade gracefully when the asset is absent.

#### Scenario: Shell renders the logo lockup and glass rail

- **WHEN** the dashboard shell is rendered
- **THEN** the navigation rail uses the frosted-glass treatment
- **AND** the scryrs logo lockup appears in the rail
- **AND** the active navigation item is indicated with the cyan accent (e.g. edge-glow or lit indicator)

#### Scenario: Shell degrades gracefully without the logo asset

- **WHEN** the logo asset is missing or fails to load
- **THEN** the shell still renders a legible scryrs text wordmark
- **AND** no broken-image placeholder is shown

### Requirement: Hotspots view is a heat leaderboard hero

The Hotspots view (the landing view) SHALL present its ranked subjects as a glass "heat leaderboard" hero in which flame intensity encodes the score, with inline event-type distribution mini-bars and outcome pulse chips, displayed above a restyled detail table. The view SHALL preserve existing behavior: ranking, sorting, navigation to subject detail, loading, empty, and error states.

#### Scenario: Top hotspots render as heat cards

- **WHEN** the Hotspots view loads with hotspot entries present
- **THEN** the top-ranked subjects render as glass heat cards
- **AND** each card's flame intensity reflects its relative score
- **AND** each card shows an inline event-type distribution and outcome indicators

#### Scenario: Existing hotspot behavior is preserved

- **WHEN** the Hotspots view is used
- **THEN** subjects remain sortable and link to the subject detail route
- **AND** the empty state and error-with-retry states still render when applicable

### Requirement: Sessions view is an activity ribbon

The Sessions view SHALL present sessions as an activity ribbon/timeline in which each session row uses the glass treatment and shows an event sparkline, with active (not-yet-ended) sessions indicated by a cyan pulse. Existing behavior SHALL be preserved: ordering by start time, navigation to session detail, loading, empty, and error states.

#### Scenario: Sessions render with sparklines and active pulse

- **WHEN** the Sessions view loads with sessions present
- **THEN** each session renders as a glass row with an event sparkline
- **AND** a session whose `endedAt` is null is indicated with a cyan pulse/active marker

#### Scenario: Existing session behavior is preserved

- **WHEN** the Sessions view is used
- **THEN** sessions remain ordered by start time and link to the session detail route
- **AND** the empty and error states still render when applicable

### Requirement: Events view is a live scrying feed

The Events view SHALL present events as a live monospace feed in which newly arriving events animate in with a brief cyan "scry" glow and are color-coded by event type. Existing behavior SHALL be preserved: pagination/cursor loading, optional session filtering, loading, empty, and error states.

#### Scenario: New events animate in with a scry glow

- **WHEN** new events appear in the Events feed
- **THEN** each new event animates in (fade-and-rise) with a brief cyan glow
- **AND** events are visually color-coded by event type

#### Scenario: Existing events behavior is preserved

- **WHEN** the Events view is used
- **THEN** pagination/cursor loading and session filtering continue to work
- **AND** the empty and error states still render when applicable

### Requirement: Detail views visualize relationships as a constellation

The Subject and Session detail views SHALL visualize related sessions/events as a constellation-style graph (nodes connected by hairlines, nodes glowing by activity) accompanied by glass stat tiles. Existing behavior SHALL be preserved: the data shown, navigation, loading, empty, and error states.

#### Scenario: Detail view renders a constellation and stat tiles

- **WHEN** a Subject or Session detail view loads with data present
- **THEN** related sessions/events are visualized as a constellation-style graph
- **AND** key metrics are presented in glass stat tiles

#### Scenario: Existing detail behavior is preserved

- **WHEN** a detail view is used
- **THEN** the same underlying data and navigation links remain available
- **AND** the loading, empty, and error states still render when applicable
