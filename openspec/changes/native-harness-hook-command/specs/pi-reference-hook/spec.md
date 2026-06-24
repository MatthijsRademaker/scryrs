## MODIFIED Requirements

### Requirement: Hook is a transport-only Pi extension in hooks/pi/

`hooks/pi/index.ts` SHALL remain an in-process Pi extension (Pi's runtime loads a module; there is no subprocess hook for Pi). It SHALL be reduced to a transport-only shim: register `session_start` and `tool_result`, resolve `session_id` from Pi's `SessionManager`, serialize the raw harness event, and hand it to `scryrs hook pi`. It SHALL NOT contain tool→event-type mapping logic.

#### Scenario: extension contains no mapping switch

- **WHEN** inspecting `hooks/pi/index.ts`
- **THEN** there is no tool→`TraceEvent` mapping switch
- **AND** translation is delegated to `scryrs hook pi`

### Requirement: Hook delegates to `scryrs hook pi` via pi.exec with --file

The shim SHALL invoke `scryrs hook pi --file <tmp>` via `pi.exec`, writing the raw event to a temp file because Pi's `exec()` opens stdin as `/dev/null`. The Pi adapter in `scryrs-adapter-harness` SHALL perform translation and persistence. The shim SHALL clean up the temp file and fail open on any error.

#### Scenario: shim forwards the raw event to the native command

- **WHEN** Pi fires `tool_result` for a tracked tool
- **THEN** the shim writes the raw event to a temp file and runs `scryrs hook pi --file <tmp>`
- **AND** removes the temp file afterward

#### Scenario: shim fails open

- **GIVEN** `scryrs` is missing or errors
- **WHEN** the shim runs
- **THEN** it logs via `console.error` and does not modify the agent-visible tool result

### Requirement: Tool events map to canonical TraceEvent families

The tool→event mapping for Pi SHALL be performed by the `pi` adapter in `scryrs-adapter-harness`: read→FileOpened, ast_grep_search→SearchRun, edit→EditMade, write→EditMade, lsp_navigation→SymbolInspected (FailedLookup on error); Bash→CommandExecuted debug-gated. `outcome` SHALL reflect the event's error state (post-execution).

#### Scenario: lsp_navigation maps per outcome

- **WHEN** the pi adapter processes an `lsp_navigation` event
- **THEN** it emits `SymbolInspected` on success and `FailedLookup` on error
