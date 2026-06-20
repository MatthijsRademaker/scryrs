## 1. Create `scryrs.json` at repository root

- [ ] 1.1 Create `scryrs.json` at the repository root with top-level keys: `manifest_version`, `trace_schema_version`, `scope`, `_anti_patterns`, `record`, and `supported_harnesses`.
- [ ] 1.2 Set `manifest_version` to `"0.1.0"` and `trace_schema_version` to `"0.1.0"` (matching `crates/scryrs-types/src/lib.rs` `SCHEMA_VERSION`).
- [ ] 1.3 Set `scope` to `"hook-interface-manifest"` with an explicit description that this manifest defines hook interfaces and record invocation only.
- [ ] 1.4 Add `_anti_patterns` section declaring the manifest is NOT an MCP descriptor, tool catalog, agent-callable surface, or tool-output rewriter.

## 2. Define the `record` invocation contract

- [ ] 2.1 Add a `record` object with `command` (`"scryrs"`) and `args` (`["record", "--stdin"]`) describing the canonical hook ingestion path.
- [ ] 2.2 Document `--file <PATH>` as an available alternate ingestion mode under `record.alternateMode`.
- [ ] 2.3 Note the deterministic output contract (stdout summary envelope, stderr rejection diagnostics, exit codes 0/1/2) inline or via reference.

## 3. Define the `supported_harnesses` array

- [ ] 3.1 Add `claude-code` entry with `referenceSource` pointing to `hooks/claude-code/scryrs-hook.mjs` (matching the `include_str!` path in `init.rs`).
- [ ] 3.2 Include `interceptedTools` listing the nine Claude Code tool names: `read`, `bash`, `grep`, `glob`, `edit`, `write`, `notebookedit`, `web_search`, `web_fetch`.
- [ ] 3.3 Include `capturedEventFamilies` listing the five canonical event families produced: `FileOpened`, `CommandExecuted`, `SearchRun`, `EditMade`, `DocRetrieved`.
- [ ] 3.4 Set `lifecycle` to `"none"` with a `limitations` note explaining PreToolUse-only behavior and unconditional Success outcome.
- [ ] 3.5 Add `pi` entry with `referenceSource` pointing to `hooks/pi/index.ts` (matching the `include_str!` path in `init.rs`).
- [ ] 3.6 Include `interceptedTools` listing the six Pi tool names: `read`, `bash`, `ast_grep_search`, `lsp_navigation`, `edit`, `write`.
- [ ] 3.7 Include `capturedEventFamilies` listing the six canonical event families produced: `FileOpened`, `CommandExecuted`, `SearchRun`, `EditMade`, `SymbolInspected`, `FailedLookup`.
- [ ] 3.8 Include `capturedLifecycleEvents` showing `SessionStart` is captured.
- [ ] 3.9 Set `lifecycle` to `"session_start_only"` with a `limitations` note explaining `SessionEnd` is deferred due to Pi's `session_shutdown` timing.

## 4. Validation

- [ ] 4.1 Verify `scryrs.json` is valid JSON.
- [ ] 4.2 Verify the manifest contains no MCP methods, tool registrations, callable surfaces, or tool-output-rewriting fields.
- [ ] 4.3 Verify `trace_schema_version` matches `crates/scryrs-types/src/lib.rs` `SCHEMA_VERSION`.
- [ ] 4.4 Verify reference source paths match the `include_str!()` paths in `crates/scryrs-cli/src/init.rs`.
- [ ] 4.5 Verify Claude Code entry claims no lifecycle events and Pi entry does not claim `SessionEnd` support.

## 5. Cross-reference

- [ ] 5.1 Confirm the manifest structure is consistent with the `scryrs-manifest` spec in `openspec/specs/scryrs-manifest/spec.md`.
- [ ] 5.2 Document that `trace-hook-contract.md` (Task 05 follow-up) must be updated to reference the real file and deprecate the provisional skeleton.