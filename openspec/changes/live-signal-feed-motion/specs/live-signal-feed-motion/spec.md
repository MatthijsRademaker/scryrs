## ADDED Requirements

### Requirement: Signals render as a newest-first live feed

The Signals view SHALL present signals as a newest-first live feed rather than a bottom-appending table. The most recently arrived signal SHALL appear at the top of the feed, and each signal SHALL render as a row with breathing room that surfaces its subject, kind, score, threshold, and delta. Existing connection states (connecting, connected, reconnecting, error), the reconnect cursor, and the empty/error states SHALL be preserved.

#### Scenario: Newest signal appears at the top

- **WHEN** a new signal is appended to the feed
- **THEN** it renders at the top of the feed
- **AND** previously shown signals move down to make room

#### Scenario: Connection and empty states are preserved

- **WHEN** the stream is connecting, reconnecting, errored, or has produced no signals
- **THEN** the corresponding connection badge, error/reconnect affordance, or empty state renders as it did before the feed redesign

### Requirement: Live signals animate in with a data-driven arrival motion

A signal that arrives while the stream is live SHALL animate in with a single signature "arrival" motion: the row enters from the top, the rows below settle to make room using spring-based layout motion, the heat indicator flares once, and the score counts up to its value before the feed settles to a calm resting state. The motion SHALL be data-driven: a larger `delta` (the amount by which the signal crossed its threshold) SHALL produce a stronger flare and settle, and the resting heat intensity SHALL scale with `score`. The motion SHALL be a single soft settle and SHALL NOT use cartoonish or oscillating bounce.

#### Scenario: A live signal ignites on arrival

- **WHEN** a signal arrives after the stream is connected (a live-tail signal)
- **THEN** its row enters from the top and the rows below settle to make room
- **AND** its heat indicator flares once and its score counts up to its value
- **AND** the feed returns to a calm, still resting state after the arrival completes

#### Scenario: Arrival magnitude scales with delta

- **WHEN** two live signals arrive with different `delta` values
- **THEN** the signal with the larger `delta` produces a visibly stronger flare and settle than the one with the smaller `delta`

#### Scenario: Resting heat scales with score

- **WHEN** a signal is at rest in the feed
- **THEN** its heat indicator intensity reflects its `score` using the shared flame heat scaling

### Requirement: Replayed history is calm and does not ignite

Signals replayed from the persisted stream on connect (the replay batch) SHALL appear without the ignition motion. Replayed signals SHALL fade in calmly so that connecting to the stream does not produce a burst of simultaneous flares. Only signals arriving after the stream is live SHALL receive the arrival motion.

#### Scenario: Replay batch fades in without flares

- **WHEN** the stream connects and replays previously persisted signals
- **THEN** those signals fade in calmly without per-signal flares or count-ups

#### Scenario: Only post-connect signals ignite

- **WHEN** the replay batch has been rendered and a new signal then arrives live
- **THEN** only the newly arrived live signal receives the arrival motion

### Requirement: The signals store distinguishes live signals from replayed history

The signals store SHALL mark each signal with whether it arrived live (after the stream opened) or as part of the replay batch, and SHALL expose ordering suitable for a newest-first feed. The flag SHALL be derived from the stream-open boundary, not from signal content.

#### Scenario: Signals are flagged by arrival phase

- **WHEN** signals are appended before or at the stream-open boundary
- **THEN** they are marked as not live (replay)
- **AND** signals appended after the stream is open are marked as live

### Requirement: Arrival motion honors reduced-motion preferences

When the operating system requests reduced motion, the live signal arrival SHALL collapse to a simple opacity fade with no flare, no overshoot, and no count-up. The reduced-motion behavior SHALL be governed by a single source of truth shared with the dashboard's existing reduced-motion handling, not a competing second system.

#### Scenario: Reduced motion collapses the arrival

- **WHEN** the user's OS prefers reduced motion and a live signal arrives
- **THEN** the signal appears with a simple opacity fade
- **AND** no flare, overshoot, layout spring, or count-up is played
