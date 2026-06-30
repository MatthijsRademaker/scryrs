# live-hotspot-polling-motion Specification

## Purpose
TBD - created by archiving change task-7160e351-6c4c-43af-b84f-22018f28d96d. Update Purpose after archive.
## Requirements
### Requirement: Hotspots view auto-refreshes in live dashboard mode

The Hotspots view SHALL periodically refresh hotspot data in live dashboard mode without requiring a full browser refresh. When the Hotspots view mounts and the dashboard is in live mode, the hotspot store SHALL begin polling `GET /api/hotspots` on a configurable interval (default 15 seconds). Polling SHALL stop when the view unmounts. Consecutive poll requests SHALL NOT overlap: if a fetch is still in flight when the next tick fires, that tick SHALL be skipped. Polling SHALL pause when the browser tab is hidden (`document.hidden === true`) and resume with a fresh fetch when the tab becomes visible, with a 500ms debounce to prevent rapid toggling.

#### Scenario: Polling starts in live mode and stops on unmount

- **GIVEN** the dashboard is in live mode
- **WHEN** the user navigates to the Hotspots view
- **THEN** the hotspot store begins periodic `/api/hotspots` fetches at the default interval
- **AND** when the user navigates away from the Hotspots view, the polling interval is cleared

#### Scenario: Polling does not run in local mode

- **GIVEN** the dashboard is in local mode
- **WHEN** the user navigates to the Hotspots view
- **THEN** the hotspot store performs a single `load()` on mount and does not start periodic polling

#### Scenario: Polling skips overlapping fetches

- **GIVEN** the hotspot store is polling
- **AND** a fetch is currently in flight
- **WHEN** the next interval tick fires
- **THEN** that tick is skipped and no concurrent request is issued

#### Scenario: Polling pauses when the tab is hidden

- **GIVEN** the hotspot store is actively polling
- **WHEN** the browser tab becomes hidden (`document.hidden` transitions to `true`)
- **THEN** the polling interval is paused
- **AND** when the tab becomes visible again, polling resumes with a fresh fetch after a 500ms debounce

### Requirement: Client-side delta is computed between successive poll snapshots

When a poll succeeds with fresh hotspot data, the hotspot store SHALL compute the delta between the new entry list and the previous entry list, keyed by the composite identity `(subjectKind, subject)`. Three classifications SHALL be derived: `entered` (entries present in the new list but not the old), `exited` (entries present in the old list but not the new), and `changed` (entries present in both lists whose `score` or `rank` differs). Unchanged entries SHALL update silently. The initial load SHALL populate entries without marking any as `entered` or `changed` — only subsequent polls SHALL produce delta classifications. Score increases SHALL be flagged separately from score decreases so that motion can distinguish upward progress from cooling.

#### Scenario: New entry is classified as entered on a subsequent poll

- **GIVEN** the hotspot store has completed at least one successful poll
- **AND** a subsequent poll returns an entry not present in the previous snapshot
- **THEN** that entry is classified as `entered`

#### Scenario: Changed score is classified as changed with direction

- **GIVEN** the hotspot store has a previous snapshot
- **AND** a subsequent poll returns the same entry with a higher `score`
- **THEN** that entry is classified as `changed` with `scoreIncreased: true`

#### Scenario: Score decrease is classified as changed without increase flag

- **GIVEN** the hotspot store has a previous snapshot
- **AND** a subsequent poll returns the same entry with a lower `score`
- **THEN** that entry is classified as `changed` with `scoreIncreased: false`

#### Scenario: Initial load does not mark entries as animation candidates

- **GIVEN** the hotspot store has not yet loaded any data
- **WHEN** the initial `load()` or first poll completes
- **THEN** entries are populated but none are classified as `entered` or `changed`

### Requirement: Hero cards animate with restrained motion-v on rank and score changes

In live dashboard mode, the top-3 hero cards in the Hotspots view SHALL use `motion-v` spring physics for data-driven motion on subsequent poll updates. Rank changes between polls SHALL drive FLIP layout reordering via the `layout` prop on `Motion` components. Score increases SHALL drive a count-up animation (rAF-based tween, ease-out cubic, capped at 800ms). New entrants on subsequent polls SHALL animate in with an entrance spring (`opacity: 0 → 1`, `y: -14 → 0`, `scale: 0.97 → 1`). The existing CSS `scry-in` entrance class SHALL NOT be applied to hero cards in live mode (to avoid double-animation with motion-v). In local mode, hero cards SHALL continue to use the `scry-in` CSS entrance as before. The `<AnimatePresence>` wrapper SHALL manage enter/exit transitions for new and dropped entries.

#### Scenario: Rank change triggers FLIP layout reorder on hero cards

- **GIVEN** the Hotspots view is in live mode with hero cards visible
- **AND** a poll returns updated data where the rank-2 and rank-3 cards have swapped positions
- **WHEN** the view updates
- **THEN** the cards reorder using spring-physics layout animation via `motion-v` `layout` prop

#### Scenario: Score increase triggers count-up on hero card

- **GIVEN** the Hotspots view is in live mode with hero cards visible
- **AND** a poll returns updated data where the rank-1 card's score increased
- **WHEN** the view updates
- **THEN** that card's score animates from its previous value to the new value via count-up tween
- **AND** the count-up completes within 800ms regardless of the score magnitude

#### Scenario: Score decrease updates silently on hero card

- **GIVEN** the Hotspots view is in live mode with hero cards visible
- **AND** a poll returns updated data where a card's score decreased
- **WHEN** the view updates
- **THEN** the score updates to the new value without a count-up or count-down animation

#### Scenario: New entrant animates in on subsequent poll

- **GIVEN** the Hotspots view is in live mode
- **AND** a poll returns a new entry that was not in the previous snapshot and ranks in the top 3
- **WHEN** the view updates
- **THEN** the new hero card enters with an entrance spring animation

#### Scenario: CSS scry-in is not used in live mode

- **GIVEN** the dashboard is in live mode
- **WHEN** the Hotspots view renders hero cards
- **THEN** the CSS `scry-in` class is not applied to hero cards
- **AND** motion-v is the sole animation mechanism for hero card entrance and reorder

#### Scenario: CSS scry-in is preserved in local mode

- **GIVEN** the dashboard is in local mode
- **WHEN** the Hotspots view renders hero cards
- **THEN** the CSS `scry-in` class is applied as before, with the index-staggered animation delay

### Requirement: Detail table highlights score changes without layout reordering

In live dashboard mode, the sortable detail table SHALL surface score changes with a brief CSS highlight on the score cell, without applying FLIP layout animation. Table rows SHALL be keyed by `(subjectKind, subject)` for stable identity. The user's current sort key and direction SHALL be preserved across poll data updates. The CSS highlight SHALL be a brief oklch color transition lasting approximately 1 second and SHALL be suppressed when the operating system requests reduced motion.

#### Scenario: Score change triggers brief highlight on table cell

- **GIVEN** the Hotspots view is in live mode with the detail table visible
- **AND** a poll returns updated data where an entry's score increased
- **WHEN** the view updates
- **THEN** that row's score cell briefly highlights with a color transition before settling to its normal appearance

#### Scenario: User sort state is preserved across polls

- **GIVEN** the user has sorted the detail table by "Score" in descending order
- **WHEN** a poll returns fresh data
- **THEN** the table continues to display entries sorted by score descending with the current sort indicator

#### Scenario: Score highlight is suppressed in reduced motion

- **GIVEN** the operating system requests reduced motion
- **WHEN** a poll returns a score change
- **THEN** the score cell updates to the new value without a highlight transition

### Requirement: Hotspots view exposes live refresh health

The Hotspots view SHALL display a live-status badge indicating the current refresh state. The store SHALL expose a `pollState` field with values: `idle` (not polling), `polling` (interval active, waiting for next tick), `updating` (fetch in flight), `stale` (last poll failed but cached data is displayed), and `error` (no data available). A `lastUpdated` relative timestamp SHALL show when the last successful poll completed. In the `stale` state, a retry affordance SHALL be shown alongside the cached data. In the `error` state (no data at all), the existing error `Alert` with retry SHALL be preserved.

#### Scenario: Live badge shows current polling state

- **GIVEN** the Hotspots view is in live mode and polling is active
- **WHEN** the poll state is `polling`, `updating`, `stale`, or `error`
- **THEN** a badge in the view header displays the current state with an appropriate visual variant

#### Scenario: Last-updated timestamp is visible

- **GIVEN** at least one successful poll has completed
- **WHEN** the Hotspots view is rendered
- **THEN** a relative timestamp (e.g., "Updated 12s ago") is displayed showing the time since the last successful poll

#### Scenario: Retry affordance shown in stale state

- **GIVEN** a previous poll succeeded and cached data is displayed
- **AND** the most recent poll failed
- **WHEN** the Hotspots view is rendered
- **THEN** the live badge shows `stale`
- **AND** a Retry button is available that triggers a manual poll
- **AND** the cached hotspot data remains visible

### Requirement: Last successful report is preserved across poll failures

When a poll fetch fails, the hotspot store SHALL preserve the last successful `HotspotsReport` in memory and continue to expose its entries through the same `entries` computed property. The `error` field SHALL reflect the most recent fetch error. A `staleError` field SHALL be set when a poll fails after at least one previous successful report, distinguishing transient poll failures from initial load failures where no data is yet available.

#### Scenario: Last good report survives poll failure

- **GIVEN** at least one poll has succeeded and populated the view with hotspot data
- **WHEN** a subsequent poll fails
- **THEN** the previously successful hotspot entries remain visible in the view
- **AND** `staleError` is set with the failure details

#### Scenario: Initial load failure shows error state

- **GIVEN** the hotspot store has never successfully loaded data
- **WHEN** the initial `load()` or first poll fails
- **THEN** `error` is set and `staleError` is not set (no cached data to display)
- **AND** the view renders the existing error `Alert` with retry

### Requirement: Existing Hotspots behavior remains intact

The polling and motion additions SHALL preserve all existing Hotspots view behaviors: subject links SHALL continue to navigate to the subject detail route; user-controlled sorting (by rank, subject, score, session count, first seen, last seen) SHALL remain functional; loading, empty, and error states SHALL remain readable; live-mode copy SHALL continue to avoid implying `.scryrs/hotspots.json` as the data source; local-mode copy SHALL continue to reference the local artifact file; and no new server-side endpoint or SSE contract SHALL be required for hotspot data.

#### Scenario: Subject links still navigate correctly in live mode with polling

- **GIVEN** the Hotspots view is in live mode with polling active
- **WHEN** the user clicks a subject link in a hero card or table row
- **THEN** navigation to the subject detail route occurs as before

#### Scenario: Sorting remains functional during polling

- **GIVEN** polling is active and the detail table is visible
- **WHEN** the user clicks a column header to sort
- **THEN** the table sorts by that column
- **AND** the sort key and direction are preserved when a subsequent poll refreshes the data

#### Scenario: Empty state renders correctly with polling

- **GIVEN** the Hotspots view is in live mode
- **AND** the server returns an empty entry list
- **WHEN** the view renders
- **THEN** the empty state is shown with live-mode-appropriate messaging

### Requirement: Hotspot polling store is covered by automated frontend tests

The hotspot store SHALL be covered by automated frontend tests using Vitest fake timers and a mocked `getHotspots` function. Tests SHALL cover: poll lifecycle (start creates interval, stop clears it, cleanup on store teardown), overlapping fetch guard (tick skipped when loading is true), error preservation (last good report survives failed polls), delta computation (entered/exited/changed classifications on ranking updates), tab-visibility pause/resume (polling pauses on `document.hidden`, resumes on visibility change), and first-load vs subsequent poll distinction (initial load does not flag entries as animation candidates).

#### Scenario: Poll lifecycle tests use fake timers

- **GIVEN** the hotspot store test suite
- **WHEN** `startPolling()` is called
- **THEN** fake timers advance and `getHotspots` is called on each interval tick
- **AND** `stopPolling()` prevents further calls

#### Scenario: Overlapping fetch guard is tested

- **GIVEN** `getHotspots` is mocked to not resolve immediately
- **AND** a poll tick has fired and the fetch is in flight
- **WHEN** the next interval tick fires
- **THEN** `getHotspots` is not called a second time

#### Scenario: Delta computation test covers ranking update

- **GIVEN** a previous snapshot with entries [A(rank 1, score 100), B(rank 2, score 80)]
- **AND** a new poll returns [B(rank 1, score 85), A(rank 2, score 100)]
- **WHEN** the delta is computed
- **THEN** entry B is classified as `changed` with rank changed and `scoreIncreased: true`
- **AND** entry A is classified as `changed` with rank changed and `scoreIncreased: false`

