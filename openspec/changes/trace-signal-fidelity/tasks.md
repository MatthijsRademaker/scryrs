## 1. Empirical groundwork (blocks everything else)

- [x] 1.1 Capture a real Pi `tool_result` payload for `ast_grep_search` and `lsp_navigation`; record the observed `input` key names in `design.md` under D4. Do not guess replacements. **Done — see D4. Found more than a field-name bug: both tools are `pi-lens` extension tools, not Pi core (`ToolName` union is `read|bash|edit|write|grep|find|ls`). `ast_grep_search` key is `pattern` (324/324); `lsp_navigation` has no single key field (`symbol` only 7/126) and is an operation dispatcher. Pi's real `grep`/`find` search tools are unmapped.**
- [x] 1.2 Verify whether Claude Code fires `PostToolUse` when a tool crashes (vs. returns a structured error). Record the finding in `design.md` under D6; if crashes do not fire, state the resulting coverage gap explicitly. **Done — see revised D6. `PostToolUse` fires ONLY on success; Claude Code has a separate `PostToolUseFailure` event. Migrating to `PostToolUse` alone would have been a no-op. Corrected to dual registration, distinguished by `hook_event_name`. Also: `PostToolUse` can `block`/`updatedToolOutput`, so non-interference now needs an explicit spec requirement.**
- [x] 1.3 Add golden fixtures under `crates/scryrs-adapter-harness/` — one captured payload per supported Pi tool and per supported Claude Code tool — so a harness-side field rename fails a test. **18 fixtures (8 Pi, 10 Claude Code) + `tests/golden_fixtures.rs`, asserting event type, subject, no-placeholder, failure outcomes, and cross-harness subject agreement.**

## 2. Fail-loud field extraction

- [x] 2.1 Add one shared key-field extraction helper in `crates/scryrs-adapter-harness/src/lib.rs` returning `Option<String>`; no placeholder, no empty-string default.
- [x] 2.2 Migrate `pi.rs` off `input_field` and `claude_code.rs` off `field` / `first_field` to the shared helper.
- [x] 2.3 On a missing key field for a supported tool: drop the event and write an unconditional diagnostic naming harness, tool, and expected key. **Deviation (see design A1): returns `Err(AdapterError::MissingField)` rather than `Ok(None)`, so `Ok(None)` keeps meaning exactly "untracked tool, pass-through". The unconditional channel is `.scryrs/hooks/<harness>-warnings.log`, not stderr — Claude Code can surface hook stderr to the model, so unconditional stderr would risk transcript pollution.**
- [x] 2.4 Confirm exit code stays 0 and stdout stays empty on the drop path (fail-open toward the agent).
- [x] 2.5 Delete `missing_input_field_yields_unknown` (`pi.rs:276`) and replace it with a test asserting drop + diagnostic + exit 0.
- [x] 2.6 Assert no `"unknown"` or empty-string subject can be constructed by either adapter.

## 3. Correct Pi field mapping

- [x] 3.1 Apply the key names observed in 1.1 to `ast_grep_search` and `lsp_navigation` in `pi.rs`.
- [ ] 3.2 Verify against a live Pi session that `SearchRun.query` and `SymbolInspected.name` carry real values.
- [x] 3.3 Update the `pi-reference-hook` mapping table to the observed keys and remove the mandated `"unknown"` fallback wording.

## 4. Subject normalization

- [x] 4.1 Normalize path subjects to repository-relative POSIX form in the adapter layer, using the repository root available from `HookContext`.
- [x] 4.2 Apply to `FileOpened.path`, `EditMade.target`, and `FailedLookup.subject`. Leave `search`, `symbol`, and `command` subjects untouched. **Deviation: `DocRetrieved` is NOT normalized — its only producer is Claude Code `WebFetch`, whose subject is a URL, not a repository path. Path-normalizing a URL would corrupt it.**
- [x] 4.3 Mark repository-external paths as external, recorded verbatim and grouped separately. Choose the mechanism without a `SCHEMA_VERSION` bump if possible; if not possible, state why in `design.md`.
- [x] 4.4 Normalize `.`/`..` segments and duplicate separators so a single file has exactly one subject spelling.
- [x] 4.5 Confirm `scryrs record --stdin` records externally-supplied pre-built events as given (normalization is a translation concern, not a store concern).
- [x] 4.6 Decide and implement: migrate existing `trace_events` subjects. **Migrate in place, on `EventStore::open`, guarded by a `subject_normalization_migrated` marker. See design A4 — running it for real caught two would-be store-corrupting bugs (container-mount root, relative store path), both now covered by regression tests.**
- [x] 4.7 Verify on the dogfooded store that the 30 known double-counted files collapse to one subject each. **Verified: 30 → 0. 166 → 23 absolute subjects (the 23 are genuinely external), 213 → 183 distinct subjects, 215 → 185 hotspot entries. `roadmap.mdx` merged from score 20 → 28.**

## 5. FailedLookup reclassification

- [x] 5.1 Pi: a failed `read` (`isError`) emits `FailedLookup` with the attempted path, not `FileOpened` + failure.
- [x] 5.2 Claude Code: a failed read derived from `tool_response` emits `FailedLookup`, matching Pi.
- [x] 5.3 ~~Keep `lsp_navigation` failure → `FailedLookup` behavior intact.~~ **Obsolete: `lsp_navigation` is dropped entirely per the 1.1 finding and the scope decision. `FailedLookup` is now sourced from failed reads on both harnesses.**
- [x] 5.4 Confirm `FileOpened` can no longer carry `Outcome::Failure`, and that nothing downstream depended on that combination.
- [x] 5.5 Re-check `memory_patch` reachability against the real store. **3 entries eligible (ratio ≥ 0.5, score ≥ 4) — the failure-ratio trigger stays reachable, so no re-gating is needed.**
- [ ] 5.6 Confirm `debugging_playbook` (`FailedLookup >= 2`) fires on real evidence. **BLOCKED on today's store, by design — see A5. The reclassification is forward-only: the migration normalizes subjects, not event types, so the 13 historical failures stay filed as `FileOpened`+`Failure`. The gate will fire as new events accumulate. Needs either newly-recorded evidence or a follow-up retro-classification migration.**

## 6. Claude Code PostToolUse migration

- [x] 6.1 Rewrite `claude_code.rs` to parse the `PostToolUse` payload. **Deviation (see revised D6): also parses `PostToolUseFailure`, and derives the outcome from `hook_event_name` rather than from `tool_response`, because `PostToolUse` fires only on success.**
- [x] 6.2 Derive a real `Outcome` from `tool_response`; remove the unconditional `Outcome::Success`.
- [x] 6.3 Change `scryrs init --agent claude-code` to register **both `PostToolUse` and `PostToolUseFailure`** in `.claude/settings.json` and remove any `PreToolUse` registration it previously wrote (Rule 7 — replace, don't coexist). Foreign `PreToolUse` entries are preserved; an emptied `PreToolUse` key is removed.
- [x] 6.4 Verify no double-counting when a settings file already carries a scryrs `PreToolUse` entry.
- [x] 6.5 Make `scryrs doctor` report a stale `PreToolUse` registration as an actionable finding. **Also reports a partial registration (present on only one of the two events). The hook command/event contract is now defined once in `init.rs` and imported by `doctor.rs`, removing a duplicated constant that could have let doctor disagree with init.**
- [x] 6.6 Verify non-interference: empty stdout, exit 0, tool result unmodified, and **no `decision` / `updatedToolOutput` emitted** — `PostToolUse` can block and replace tool output, which `PreToolUse` could not, so this is now an explicit spec requirement rather than an incidental property.
- [x] 6.7 Update `hooks/claude-code/README.md` — replace the "PreToolUse Only — Outcome Is Always Success" section with the `PostToolUse` contract and the 1.2 crash-coverage finding.

## 7. Search-capture decision (D7)

- [x] 7.1 Choose one of the three D7 options and record the decision with its rationale in `design.md`. **The premise was wrong. The corpus is 100% Pi with zero `bash` AND zero `grep`/`find` events — Pi's real search tools were in use and the adapter silently discarded them. The search gap was an adapter bug, not the observer-first Bash policy. Decision: no Bash capture change.**
- [x] 7.2 Implement the chosen option, or record explicitly that no capture change is made. **No capture change. The gap is closed by §3 (mapping `grep`/`find`/`ast_grep_search` on `pattern`). Claude Code's `Grep`/`Glob`/`WebSearch` were already correct.**
- [x] 7.3 ~~Correct `vision.md`~~ **No correction needed. "Repeated search terms" becomes supportable for the first time rather than needing withdrawal — correcting it downward would have documented a bug as a limitation.**
- [x] 7.4 ~~Correct `roadmap.mdx` Phase 1/Phase 8~~ **No change needed on this account; the `roadmap-truth` requirements are unaffected.**
- [x] 7.5 Capture `FileOpened` session co-occurrence as a named future thread. **Added to `roadmap.mdx` as a standalone spike with its open questions.** It survives on its own merits (386 events over 30 sessions, fully captured, unmined, answers "what should I load alongside this?"), not as a fallback for a gap that no longer exists.

## 8. Verification

- [x] 8.1 `scripts/precommit-run` passes. **Exit 0, no failures.**
- [x] 8.2 Regenerate affected snapshots per `testing-this-repo`; review the ranking diff deliberately. **Two snapshots changed (`hotspot_stdout`, `hotspot_artifact`); the diff was exactly the `FailedLookup` fixture subject and its `subjectKind` — no score or ranking drift — and was reviewed line-by-line before accepting.**
- [x] 8.3 Re-run `scryrs hotspots .` against the dogfooded store and confirm normalization collapsed the split subjects. **Confirmed — see 4.7.**
- [x] 8.4 Confirm every proposal kind's reachability against the real store. **Measured on the migrated store: `skill` 13 eligible, `memory_patch` 3 eligible, `debugging_playbook` 0. See design A5.**
- [x] 8.5 `scripts/verify-trace-capture` passes on both harnesses. **All three lanes pass (Claude Code, Pi, Init hooks). Four fixtures asserted the old contract and were updated: `ast_grep_search` key, `lsp_navigation` removal, failed-read→`FailedLookup`, and the `PreToolUse`→dual post-tool registration.**
- [ ] 8.6 Manual cross-harness check: one real Claude Code session and one real Pi session produce events with normalized subjects, real query/symbol values, and correct outcomes.
- [x] 8.7 Update `.devagent/docs/docs/trace-hook-contract.md` and `hotspots.md` to match the shipped behavior. **Dual post-tool registration, non-interference restraint, corrected Pi tool set, plus the new `external_file` subject kind, path normalization, and failed-lookup grouping in `hotspots.md`.**
