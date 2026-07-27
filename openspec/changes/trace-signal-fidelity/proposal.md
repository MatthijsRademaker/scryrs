## Why

scryrs's evidence base is not fit for the purpose the vision claims. Measured against the dogfooded store at `.scryrs/scryrs.db` (30 sessions, 538 events, verified 2026-07-25):

| EventType | Count | Share |
|---|---|---|
| `FileOpened` | 386 | 71.7% |
| `EditMade` | 115 | 21.4% |
| `SessionStart` | 32 | 5.9% |
| `SymbolInspected` | 3 | 0.6% |
| `SearchRun` | 2 | 0.4% |
| `FailedLookup` | 0 | 0.0% |
| `CommandExecuted` | 0 | 0.0% |
| `DocRetrieved` | 0 | 0.0% |

`vision.md` promises detection of "hot concepts, missing docs, context churn, repeated search terms, fragile code areas, undocumented decision zones." What is actually captured is file-open frequency. Four concrete defects cause this, each verified against the tree and the live store:

**1. Subject strings are never normalized.** No normalization exists in `scryrs-adapter-harness`, `scryrs-core::store`, or `scryrs-core::ingestion` — the subject is whatever path string the agent happened to type. In the dogfooded store 166 of 506 subject-bearing events (33%) carry absolute paths and 340 carry repository-relative paths, **within the same tool** (`read`: 134 absolute, 252 relative). 30 distinct files are double-counted under two subject strings, splitting their scores. Examples: `.devagent/docs/docs/roadmap.mdx`, `.pi/skills/docs-writer/SKILL.md`, `.pi/rules/runtime-environment.md`.

**2. Silent placeholder defaults hide total extraction failure.** `pi.rs::input_field` returns `"unknown"` when a field is absent (`crates/scryrs-adapter-harness/src/pi.rs:28`); `claude_code.rs::field` returns `""` (`crates/scryrs-adapter-harness/src/claude_code.rs:26`). Two different silent defaults for the same failure mode, neither of which fails. Consequence: **all 5 non-file events in the store have subject `"unknown"`** — 2/2 `SearchRun` and 3/3 `SymbolInspected`. A 100%-failure-rate field-mapping bug survived a month of dogfooding because the placeholder made it look like data. This directly violates AGENTS.md Rule 5 (Fail Fast) and Rule 6 (No Defensive Noise), and `openspec/specs/pi-reference-hook/spec.md` currently *mandates* the placeholder plus a warning that the Rust implementation never emits.

**3. Failed file reads are misclassified.** A `read` of a nonexistent path is recorded as `FileOpened` with `Outcome::Failure`, not as `FailedLookup`. The Pi adapter emits `FailedLookup` only for `lsp_navigation` errors (`pi.rs:88`). All 13 failure outcomes in the store are exactly this case — an agent guessing a wrong path — which is the single most routing-relevant signal available and is currently filed under the wrong event family. Because `debugging_playbook` gates on `FailedLookup >= 2` (`scryrs-curator/src/lib.rs:70`), that proposal kind is unreachable despite the evidence for it existing.

**4. Claude Code cannot carry outcomes at all.** The integration hooks `PreToolUse`, which fires before execution, so `Outcome` is unconditionally `Success` (`crates/scryrs-adapter-harness/src/claude_code.rs:1-6`, `hooks/claude-code/README.md:108`). No `FailedLookup`, no failure ratio, no outcome signal of any kind is structurally possible on the harness with the widest reach. `memory_patch` (failure-ratio >= 0.5) and `debugging_playbook` are therefore both unreachable for every Claude Code user.

Every downstream surface — hotspots, graph, routes, proposals, publishing — consumes this evidence. Fixing retrieval or authoring agent-facing skills on top of a corpus that is 93% "which files were opened" produces confident routing toward no information. This change fixes the signal so those surfaces have something to carry.

## What Changes

### 1. Canonical subject normalization

- Normalize file-path subjects to repository-relative POSIX form at a single point in the ingestion path before the event is persisted, so `(subject_kind, subject)` grouping is stable regardless of how the agent addressed the file.
- Paths outside the repository root are recorded verbatim and marked as external rather than silently rewritten or dropped.
- Normalization applies to `FileOpened`, `EditMade`, `FailedLookup`, and `DocRetrieved` path subjects. `search`, `symbol`, and `command` subjects are not paths and are not normalized.
- Provide a one-time migration for existing stores so historical double-counted subjects collapse, or explicitly document that historical rows are left as-is behind a schema-meta marker.

### 2. Fail-loud field extraction

- Remove the `"unknown"` and `""` placeholder defaults. A supported tool whose key input field is absent is a contract violation, not a data point.
- On a missing key field, drop the event rather than record a placeholder, and emit a diagnostic on stderr. The hook remains fail-open toward the agent: the tool proceeds, exit stays 0, agent-visible output is untouched.
- Unify the two adapters on one shared extraction helper so a third harness cannot introduce a third silent default.
- Replace the enshrining test `missing_input_field_yields_unknown` (`pi.rs:276`) with one asserting the drop-plus-diagnostic behavior.

### 3. Correct the Pi field mapping empirically

- Capture real Pi `tool_result` payloads for `ast_grep_search` and `lsp_navigation` and record the actual `input` key names. The current spec asserts `input.query` and `input.symbol`; the 100% placeholder rate proves at least one is wrong. Field names are corrected from observed payloads, not guessed.
- Add a golden fixture per supported Pi tool captured from a real payload, so a Pi-side rename fails a test instead of silently degrading to a placeholder.

### 4. Failed lookups become FailedLookup

- A failed `read` (Pi `isError`) emits `FailedLookup` with the attempted path as subject, not `FileOpened` with a failure outcome.
- `FailedLookup` becomes the event family for "agent addressed something that does not exist," covering both failed path reads and failed symbol navigation.
- `debugging_playbook` and `memory_patch` become reachable from real evidence as a consequence, without changing their thresholds.

### 5. Claude Code gains an outcome-bearing event path

- Move the Claude Code integration from `PreToolUse` to `PostToolUse`, which carries `tool_response` and therefore a real outcome.
- Derive `Outcome::Failure` from the tool response and route failed reads to `FailedLookup`, matching the Pi behavior.
- `scryrs init --agent claude-code` writes the `PostToolUse` registration; existing `PreToolUse` registrations written by prior installs are replaced, not left alongside (AGENTS.md Rule 7).
- Non-interference is preserved: `PostToolUse` still writes nothing to stdout, never modifies the tool result, and always exits 0.

### 6. Search-capture decision, made explicitly

- Record a decision on Bash/search capture in `design.md`. In this repository the dominant search modality is `rg` through Bash, which the observer-first default excludes — which is why `SearchRun` is 0.4% of the corpus. Either search evidence is obtainable or the "repeated search terms" claim in `vision.md` must be withdrawn.
- Whichever way it resolves, `vision.md` and `roadmap.mdx` are corrected so the documented detection claims match what the corpus can support.

## Impact

- `(subject_kind, subject)` becomes a stable grouping key; hotspot scores stop splitting across path spellings. Existing hotspot rankings will change — this is the point, not a regression.
- Field-mapping breakage becomes loud and testable instead of appearing as a `"unknown"` row.
- `debugging_playbook` becomes reachable; `memory_patch` becomes reachable on Claude Code.
- Claude Code trace events gain real outcomes, at the cost of moving to a different hook event and replacing the installed registration.
- `openspec/specs/pi-reference-hook/spec.md` loses its mandated `"unknown"` placeholder behavior; `claude-code-reference-hook` loses its `PreToolUse`-only framing.
- No change to `SCHEMA_VERSION` unless the migration decision in `design.md` requires one; the envelope and payload shapes are unchanged by this proposal.
