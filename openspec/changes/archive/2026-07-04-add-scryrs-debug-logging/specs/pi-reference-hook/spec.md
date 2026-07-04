## ADDED Requirements

### Requirement: Pi hook debug logging is opt-in through SCRYRS_DEBUG

The Pi reference hook SHALL emit additional diagnostic logs only when `SCRYRS_DEBUG` is set to a non-empty value in the hook runtime environment. With `SCRYRS_DEBUG` unset, the hook SHALL preserve existing behavior: no new debug output, same TraceEvent mapping, same `scryrs record --stdin` invocation, same fail-open behavior, and same `undefined` handler return value.

#### Scenario: Debug disabled preserves quiet behavior

- **WHEN** the Pi hook handles `session_start` or `tool_result` with `SCRYRS_DEBUG` unset
- **THEN** it does not emit `[scryrs]` debug lines
- **AND** it still records supported trace events exactly as before
- **AND** it still returns `undefined` from the `tool_result` handler

#### Scenario: Debug enabled logs hook load

- **WHEN** the Pi hook module is loaded with `SCRYRS_DEBUG` set
- **THEN** it emits a `[scryrs]` debug line indicating the hook loaded
- **AND** the line includes the generated session identifier

### Requirement: Pi hook debug logs expose pipeline stage breadcrumbs

When debug logging is enabled, the Pi hook SHALL log single-line breadcrumbs for hook lifecycle and event handling stages, including `session_start`, every observed `tool_result`, tracked/untracked tool decision, input key list, field fallback, TraceEvent send attempt, record subprocess result, non-zero record exit, and record exec error. Debug logs SHALL use the stable `[scryrs]` prefix and collapse or truncate multi-line values so each debug record remains one physical line.

#### Scenario: Session start debug breadcrumb is emitted

- **WHEN** Pi fires `session_start` with `SCRYRS_DEBUG` set
- **THEN** the hook emits a `[scryrs]` debug line indicating `session_start` was observed
- **AND** the line identifies that a `SessionStart` event is being sent to `recordEvent`

#### Scenario: Tracked tool debug breadcrumb is emitted

- **WHEN** Pi fires `tool_result` for a tracked tool with `SCRYRS_DEBUG` set
- **THEN** the hook emits a `[scryrs]` debug line containing the tool name, `tracked=true`, `is_error`, and available input keys
- **AND** the hook emits a later `[scryrs]` debug line identifying the mapped TraceEvent type being sent to `scryrs record --stdin`

#### Scenario: Untracked tool debug breadcrumb is emitted

- **WHEN** Pi fires `tool_result` for an untracked tool with `SCRYRS_DEBUG` set
- **THEN** the hook emits a `[scryrs]` debug line containing the tool name, `tracked=false`, and available input keys
- **AND** the hook does not construct or send a TraceEvent for that untracked tool

#### Scenario: Missing mapped field debug breadcrumb includes available keys

- **WHEN** a tracked tool event is missing the field expected by the hook mapping
- **AND** `SCRYRS_DEBUG` is set
- **THEN** the hook emits a `[scryrs]` debug line identifying the tool name, missing field, available input keys, and fallback value
- **AND** existing fallback behavior still emits the event with `unknown` as the mapped subject value

#### Scenario: Record subprocess result is visible from hook debug logs

- **WHEN** the hook invokes `scryrs record --stdin` with `SCRYRS_DEBUG` set
- **THEN** the hook emits a `[scryrs]` debug line with subprocess exit code, killed status, and truncated stdout/stderr previews
- **AND** successful record-side debug output from the child process is visible through the hook debug log path

### Requirement: Pi hook debug wire inspection is explicit and bounded

The Pi hook SHALL provide deeper wire inspection only for explicit `SCRYRS_DEBUG` modes beyond the default debug value. `SCRYRS_DEBUG=wire` SHALL include sanitized observed input previews. Any raw inspection mode SHALL cap output length and SHALL NOT be required for normal verification. Default `SCRYRS_DEBUG=1` SHALL NOT dump full `event.content`, `event.details`, file bodies, or uncapped raw input.

#### Scenario: Default debug logs keys not full content

- **WHEN** the Pi hook handles a tool result with `SCRYRS_DEBUG=1`
- **THEN** debug logs include the available `event.input` keys
- **AND** debug logs do not dump full `event.content` or `event.details`

#### Scenario: Wire mode includes sanitized input preview

- **WHEN** the Pi hook handles a tool result with `SCRYRS_DEBUG=wire`
- **THEN** debug logs include a bounded sanitized preview of observed input fields useful for field-name verification
- **AND** any multi-line preview content is collapsed or truncated to one physical debug line

### Requirement: Pi hook debug logging remains non-interfering and fail-open

Debug logging SHALL NOT change the hook's non-interference or fail-open contract. Logging failures SHALL NOT throw from `session_start` or `tool_result`; record subprocess failures SHALL still be caught and logged; the original Pi tool result SHALL remain unchanged.

#### Scenario: Debug logging does not mutate tool result

- **WHEN** a tracked `tool_result` event is handled with `SCRYRS_DEBUG` set
- **THEN** the handler returns `undefined`
- **AND** the original event `input`, `content`, `details`, and `isError` values remain unchanged

#### Scenario: Record exec failure remains fail-open with debug enabled

- **WHEN** `pi.exec` throws while `SCRYRS_DEBUG` is set
- **THEN** the hook logs the exec error through the existing fail-open path plus debug context
- **AND** the handler does not throw
- **AND** the original tool result remains available to the agent unchanged
