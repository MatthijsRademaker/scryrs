## 1. Create reference hook source and directory

- [ ] 1.1 Create `hooks/pi/` directory at repository root
- [ ] 1.2 Create `hooks/pi/index.ts` with the default extension factory function importing from `@earendil-works/pi-coding-agent`
- [ ] 1.3 Add `crypto.randomUUID()` call on module load to generate a session-scoped `session_id`
- [ ] 1.4 Subscribe to Pi `session_start` event and emit `SessionStart` TraceEvent via `pi.exec('scryrs', ['record', '--stdin'], ...)`
- [ ] 1.5 Subscribe to Pi `tool_result` event with an async handler
- [ ] 1.6 Define the tool filter set: `new Set(['read', 'bash', 'ast_grep_search', 'lsp_navigation', 'edit', 'write'])`

## 2. Implement tool-to-TraceEvent mapping

- [ ] 2.1 Implement `read` → `FileOpenedPayload { path: event.input.path }`
- [ ] 2.2 Implement `bash` → `CommandExecutedPayload { command: event.input.command }`
- [ ] 2.3 Implement `ast_grep_search` → `SearchRunPayload { query: event.input?.query ?? 'unknown' }` with defensive access
- [ ] 2.4 Implement `edit` → `EditMadePayload { target: event.input.path }`
- [ ] 2.5 Implement `write` → `EditMadePayload { target: event.input.path }`
- [ ] 2.6 Implement `lsp_navigation` → conditional: `SymbolInspectedPayload { name }` on success, `FailedLookupPayload { subject }` on `event.isError` — with defensive field access (`event.input?.symbol ?? 'unknown'`)
- [ ] 2.7 Determine Outcome: `Outcome::Success` or `Outcome::Failure { reason }` from `event.isError`

## 3. Implement JSONL serialization and subprocess delegation

- [ ] 3.1 Construct the `TraceEvent` envelope with fields: `schema_version: "0.1.0"`, `timestamp: new Date().toISOString()`, `session_id` (from module scope), `event_type`, `tool_name`, `payload`, `outcome`
- [ ] 3.2 Serialize the event to a single-line JSON string (`JSON.stringify(event) + '\n'`)
- [ ] 3.3 Pass the JSONL string as stdin input to `pi.exec('scryrs', ['record', '--stdin'], { input: jsonlString, timeout: 5000 })`

## 4. Implement fail-open error handling

- [ ] 4.1 Wrap the entire subprocess call in try-catch
- [ ] 4.2 Log any failure (missing binary, non-zero exit, timeout, I/O error) via `console.error` with enough context to identify the tracing gap
- [ ] 4.3 Return `undefined` from the `tool_result` handler unconditionally — never modify `event.content`, `event.details`, or `event.isError`
- [ ] 4.4 Ensure the handler never calls `pi.registerTool()` or any registry-modifying API

## 5. Write companion documentation

- [ ] 5.1 Create `hooks/pi/README.md` with install instructions for consumers (copy to `~/.pi/agent/extensions/` or `.pi/extensions/`)
- [ ] 5.2 Document the full tool-to-TraceEvent mapping table
- [ ] 5.3 Document the assumed input field names for `ast_grep_search` and `lsp_navigation` with a note that consumers should verify against their Pi tool definitions
- [ ] 5.4 Document the `write` → `EditMade` mapping decision and why no `WriteMade` variant exists
- [ ] 5.5 Document the `lsp_navigation` success/failure conditional mapping
- [ ] 5.6 Document the fail-open guarantee and that scryrs must be on PATH
- [ ] 5.7 Document the deferred `SessionEnd` behavior

## 6. Verification

- [ ] 6.1 Confirm no `.pi/extensions/` consumer config files are present in the change diff
- [ ] 6.2 Confirm `hooks/pi/index.ts` does not call `pi.registerTool()`, `pi.registerCommand()`, `pi.setActiveTools()`, or any tool-registry API
- [ ] 6.3 Confirm the handler returns `undefined` (no return statement that patches tool result)
- [ ] 6.4 Confirm all six tool names are in the filter set and each maps to a documented TraceEvent family
- [ ] 6.5 Document manual verification steps for consumers: install hook, trigger each of the six tools, confirm `.scryrs/events.jsonl` contains corresponding events, confirm tool output unchanged with scryrs missing