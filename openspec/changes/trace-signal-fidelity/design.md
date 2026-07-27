## Context

scryrs captures trace events through two harness adapters in `crates/scryrs-adapter-harness/`:

```
  Claude Code                              Pi
  ═══════════                              ══
  PreToolUse                               tool_result
  (before execution)                       (after execution)
      │ tool_input only                        │ input + isError
      ▼                                        ▼
  claude_code.rs                           pi.rs
  field()      → ""                        input_field() → "unknown"
      │                                        │
      └──────────────┬─────────────────────────┘
                     ▼
              build_event()  ← no normalization
                     ▼
        scryrs-core::store::insert  ← no normalization
                     ▼
              trace_events (subject column, verbatim)
                     ▼
        scryrs-core::scoring  ← groups by (subject_kind, subject)
```

The store is the single upstream of every product surface: hotspots → graph → routes → proposals → publish. There is exactly one place where subject strings enter the system and exactly one place they are grouped, and nothing normalizes them in between.

Measurements below are from the dogfooded store at `.scryrs/scryrs.db`, read-only, on 2026-07-25 at commit `d7d903c` — 30 sessions, 538 events, all Pi-sourced (tool names `read`, `edit`, `write`, `ast_grep_search`, `lsp_navigation`).

## Goals / Non-Goals

**Goals:**
- One canonical subject form for path subjects, applied at one point.
- Extraction failures are loud, testable, and impossible to mistake for data.
- Pi field names verified against real payloads rather than asserted.
- "Agent addressed something that does not exist" lands in `FailedLookup` regardless of harness or tool.
- Claude Code can express a failure outcome at all.
- The documented detection claims match what the corpus can support.

**Non-Goals:**
- No semantic ranking, embedding, or LLM interpretation of events (Phase 9 boundary).
- No new event types. `FailedLookup`, `SearchRun`, `CommandExecuted`, `DocRetrieved` all already exist in `scryrs-types`.
- No change to scoring weights or the ranking tie-break chain (`hotspot-report` spec is authoritative and untouched here).
- No hook that modifies agent-visible tool results or rewrites commands (observer-first boundary, `trace-hook-contract` Requirement "Non-interference and fail-open rules").
- No context injection into agent prompts — that is the separate spike recorded in `roadmap.mdx`.
- No retrieval, skill, or publishing work — that is `agent-skill-surface`.

## Decisions

### D1. Normalize in the adapter layer, not the store

Normalization needs the repository root to compute a relative path. `HookContext` already carries the resolved store path and cwd (`crates/scryrs-adapter-harness/src/lib.rs:24`), so the adapter layer has the root; `scryrs-core::store` receives an already-built `TraceEvent` and would have to re-derive it.

Putting it in the adapter also keeps `scryrs record --stdin` honest: events arriving as pre-built JSON from an external producer are recorded as given rather than silently rewritten by the store. The normalization contract belongs to translation, which is where harness-specific knowledge already lives.

**Rejected:** normalizing in `scryrs-core::scoring` at read time. It would leave the store holding two spellings of the same subject, so every consumer would need to re-implement the normalization, and `scryrs-trace-query` would disagree with `scryrs hotspots`.

### D2. Paths outside the repository are marked, not rewritten or dropped

A `read` of `/etc/hosts` or `~/.claude/CLAUDE.md` is real agent behavior and legitimate evidence. Rewriting it to a `../../../` relative path produces a meaningless subject; dropping it loses signal.

Such subjects are recorded verbatim, flagged as external, and are grouped separately from repository-relative subjects. The exact flag mechanism (a `subject_kind` variant vs. a payload field) is settled in tasks — it must not require a `SCHEMA_VERSION` bump if avoidable, since the envelope is stable.

Note this is not hypothetical: the store's failure-bearing entries include `/Users/matthijsrademaker/repos/personal/scryrs/.devagent/doc_build/architecture.md` — inside the repo but in a build directory — and the user's own global instruction files are read every session.

### D3. Drop-and-diagnose beats placeholder, and beats hard failure

Three options for a missing key input field:

| | Placeholder (today) | Hard error | Drop + diagnose |
|---|---|---|---|
| Corrupts corpus | yes — 5/5 events | no | no |
| Detectable | no | yes | yes |
| Risks agent interference | no | **yes** — nonzero exit | no |
| Rule 5 compliant | no | yes | yes |

Hard failure is wrong here specifically because the hook is an observer. `trace-hook-contract` requires fail-open toward the agent, and `hooks/pi/index.ts:17` states the hook "fails open when scryrs is unavailable." A nonzero exit from a trace hook must never be able to disturb a coding session.

So: return `Option::None` from translation (the existing pass-through path for untracked tools), and write a diagnostic to stderr. Exit stays 0. The event is absent rather than wrong — absent is recoverable, wrong is not.

The diagnostic must be unconditional, not gated on `SCRYRS_DEBUG`. Gating it would reproduce exactly the failure mode this change exists to fix: a contract violation that nobody sees.

### D4. Field names are discovered, not guessed — RESOLVED 2026-07-25

`openspec/specs/pi-reference-hook/spec.md` asserted `query ← event.input?.query` for `ast_grep_search` and `symbol ← event.input?.symbol` for `lsp_navigation`. The store showed a 100% placeholder rate on both. The empirical pass found something larger than a wrong field name.

**Sources of truth used:** Pi `0.81.1` installed at `~/.local/share/fnm/node-versions/v24.11.0/installation/lib/node_modules/@earendil-works/pi-coding-agent` (typed input schemas in `dist/core/tools/*.d.ts`, authoritative tool list in `dist/core/tools/index.d.ts`), plus 450 real tool-call payloads extracted from Pi session transcripts under `~/.pi/agent/sessions/`.

#### Finding 1 — `ast_grep_search` and `lsp_navigation` are not Pi tools

Pi's authoritative tool set is a closed union in `dist/core/tools/index.d.ts`:

```ts
export type ToolName = "read" | "bash" | "edit" | "write" | "grep" | "find" | "ls";
```

Neither `ast_grep_search` nor `lsp_navigation` appears, and neither appears anywhere in Pi's CHANGELOG across its full history (0.10.0 → 0.81.1). They were never core Pi tools.

They come from **`pi-lens`**, a third-party Pi extension installed at `~/.pi/agent/npm/node_modules/pi-lens`. Its skills document a wider surface (`l`, `ast_grep_replace`, `ast_grep_outline`, `ast_grep_dump`, `lsp_diagnostics`) and note the tools are "registered but inactive by default."

So the scryrs Pi adapter maps two tools from a user-specific extension while ignoring Pi's own search tools. This — not a field-name typo — is the root cause of `SearchRun` being 0.4% of the corpus.

#### Finding 2 — Pi's real search tools are unmapped

Verified typed schemas (required fields in bold):

| Pi core tool | Input schema | scryrs mapping today |
|---|---|---|
| `read` | **`path`**, `offset?`, `limit?` | `FileOpened` ✓ correct |
| `edit` | **`path`**, **`edits[]`** | `EditMade` ✓ correct |
| `write` | **`path`**, **`content`** | `EditMade` ✓ correct |
| `bash` | **`command`**, `timeout?` | `CommandExecuted` ✓ correct |
| `grep` | **`pattern`**, `path?`, `glob?`, `ignoreCase?`, `literal?`, `context?`, `limit?` | **unmapped — falls through** |
| `find` | **`pattern`**, `path?`, `limit?` | **unmapped — falls through** |
| `ls` | `path?`, `limit?` | **unmapped — falls through** |

`grep` and `find` both carry a required `pattern` and are exactly the `SearchRun` evidence the corpus lacks. They currently hit the `_ => return Ok(None)` pass-through arm.

#### Finding 3 — observed key names for the pi-lens tools

From 324 real `ast_grep_search` payloads:

| key | presence |
|---|---|
| `pattern` | **324/324 (100%)** |
| `lang` | 324/324 (100%) |
| `paths` | 296/324 (91%) |
| `context` | 265/324 (81%) |
| `selector`, `maxMatches`, `skip`, `groupByFile` | ≤1% each |

The key field is **`pattern`**. `query` never appears in any payload.

From 126 real `lsp_navigation` payloads:

| key | presence |
|---|---|
| `operation` | **126/126 (100%)** |
| `filePath` | 87/126 (69%) |
| `query` | 64/126 (50%) |
| `path` | 28/126 (22%) |
| `maxResults` | 21/126 (16%) |
| `exactMatch` | 15/126 (11%) |
| `line` | 12/126 (9%) |
| **`symbol`** | **7/126 (5%)** |
| `character` | 4/126 (3%) |

`operation` values observed: `documentSymbol` (44), `workspaceSymbol` (43), `findSymbol` (21), `hover` (6), `references` (5), `workspaceDiagnostics` (4), `capabilities` (2), `definition` (1).

`symbol` — the key the spec asserts — is present in **5%** of calls. `lsp_navigation` is not a symbol-lookup tool; it is an **operation dispatcher**. There is no single key field: a symbol name (`query`) is present in only 50% of calls, a file path (`filePath` or `path`) in 91%, and 11/126 calls carry no path at all (`workspaceSymbol`, `workspaceDiagnostics`, `capabilities`).

#### Consequences for this change

- The `pi-reference-hook` spec's "six named Pi tools" filter and its mapping table are both wrong and must be rewritten against the verified core tool set.
- `ast_grep_search` → `SearchRun` must read `pattern`, not `query`.
- `lsp_navigation` has no single key field. `SymbolInspected` requires a `name`, which is available in at most 50% of calls. A per-`operation` mapping is required, or the tool is dropped from scope.
- `grep` and `find` must be mapped to `SearchRun` on `pattern` — this is where the missing search signal actually lives.
- Because `lsp_navigation` is the *only* current source of `FailedLookup` and it is an extension tool, `FailedLookup` has **no source at all** on a stock Pi install. D5's reclassification of failed `read` is therefore not an improvement but the only way `FailedLookup` is reachable on Pi.
- Whether scryrs maps third-party extension tools at all is a scope question, recorded in Open Questions.

### D5. FailedLookup is the "does not exist" family, across tools

Today `FailedLookup` means "failed symbol navigation." Widening it to "the agent addressed a subject that does not exist" is a semantic decision with real consequences: it changes what `debugging_playbook` and `memory_patch` fire on.

It is the right call because it matches what the evidence actually is. All 13 failure outcomes in the store are an agent reading a path that is not there:

```
ratio  score  subject
1.00     3    .pi/swarm-pi-default-setup/prompts/swarm-plan.md
1.00     3    .pi/rules/README.md
1.00     3    …/task-795d99eb…/proposal.md
0.50     8    .pi/prompts/swarm-reviewer-review.md
0.50     8    .pi/prompts/swarm-lead-dev-review.md
0.50     6    .pi/prompts/docs-review.md
…
```

This is the highest-value routing signal scryrs has: it is a direct, evidence-backed statement that an agent expected a file at a path where none exists. Filing it as `FileOpened` + `Outcome::Failure` buries it in the 386-event file-open bucket and leaves `debugging_playbook` starved.

Consequence to accept: after this change `FileOpened` events are all successes. `Outcome::Failure` on `FileOpened` becomes unreachable, and the failure-ratio input to `memory_patch` shifts from file-read failures to whatever else fails. Task work must confirm `memory_patch` still has a reachable trigger, and re-gate it on `FailedLookup` if not — the goal is reachability from real evidence, not preserving the current threshold arithmetic.

### D6. Claude Code registers on both PostToolUse and PostToolUseFailure — REVISED 2026-07-25

The original decision — "move to `PostToolUse`, which carries `tool_response` and therefore a real outcome" — was **wrong**, and the empirical pass caught it before implementation.

**Source:** Claude Code hooks documentation, `https://code.claude.com/docs/en/hooks`, verified 2026-07-25.

`PostToolUse` fires **only after a tool call succeeds**. Claude Code has a *separate* event, `PostToolUseFailure`, that fires after a tool call fails. The documented lifecycle is explicit:

| Event | Condition |
|---|---|
| `PreToolUse` | Before a tool call |
| `PostToolUse` | After a tool call **succeeds** |
| `PostToolUseFailure` | After a tool call **fails** |

So migrating to `PostToolUse` alone would reproduce exactly the defect being fixed: every event would still carry `Outcome::Success`, because failures would never reach the hook. It would have been a no-op dressed as a fix.

**Corrected decision:** `scryrs init --agent claude-code` registers the same `scryrs hook claude-code` command on **both** events:

- `PostToolUse` → `Outcome::Success`
- `PostToolUseFailure` → `Outcome::Failure`, and a failed `Read` maps to `FailedLookup` per D5

The two events are mutually exclusive per tool call, so dual registration does not double-count. The adapter distinguishes them by the `hook_event_name` field present on every hook payload, rather than by inferring an outcome from response contents.

Verified payload fields available to the adapter:
- Common: `session_id`, `prompt_id`, `transcript_path`, `cwd`, `permission_mode`, `hook_event_name`, `agent_id`, `agent_type`
- Tool events: `tool_name`, `tool_input`, `tool_use_id`
- `PostToolUse` only: `tool_response`

Both events support tool-name matchers, so the existing tool filter continues to work unchanged.

**Non-interference requires active restraint here.** `PostToolUse` supports `decision: "block"` with a `reason`, and `updatedToolOutput`, which *replaces the tool's result before Claude sees it*. `PreToolUse` had no such power in scryrs's usage. The adapter must emit neither field — no `decision`, no `updatedToolOutput`, empty stdout, exit 0. This is now an explicit spec requirement rather than an incidental property of the chosen hook event.

Costs, stated plainly:
- **Blocking semantics are lost.** `PreToolUse` can deny a tool call. scryrs never used that, so nothing is given up.
- **Events arrive later.** Irrelevant for an offline analysis corpus.
- **Two registrations instead of one**, so `init`, `doctor`, and the migration path all have to handle a pair rather than a single entry.
- Existing installs must be migrated; `scryrs doctor` should report a stale `PreToolUse` registration.

The original worry — "a tool that crashes may not fire `PostToolUse` at all" — is resolved: `PostToolUseFailure` is the documented path for failures, so there is no coverage gap to accept.

**Adjacent opportunity, deliberately out of scope:** the documented event list includes `SessionStart` and `SessionEnd`. The Claude Code integration currently emits no session demarcation at all (`hooks/claude-code/README.md`), while `trace-hook-contract` requires first-class `SessionStart`/`SessionEnd` events. That gap is real and now known to be closable, but it is not in this change's scope and should be raised as separate work rather than absorbed here.

### D7. Search capture — RESOLVED 2026-07-25: it was a bug, not a policy trade-off

The original framing of this decision was **wrong**, and the corpus disproves it.

It claimed `SearchRun` was 0.4% because of the intersection of two deliberate decisions — Bash captured only under `SCRYRS_DEBUG`, and agents in this repository instructed to search with `rg` through Bash — concluding that "the dominant search modality is excluded by policy" and `vision.md`'s "repeated search terms" claim could not be supported.

That second premise was inferred from the repository's agent instructions, not measured. The corpus contradicts it. Tool-name distribution across all 538 events:

| tool_name | count |
|---|---|
| `read` | 386 |
| `edit` | 82 |
| `write` | 33 |
| *(none — SessionStart)* | 32 |
| `lsp_navigation` | 3 |
| `ast_grep_search` | 2 |

The corpus is **100% Pi**, and contains **zero `bash` events and zero `grep`/`find` events**. Not because those tools went unused — `grep` and `find` are Pi core tools and were in use — but because the Pi adapter mapped neither, so `_ => return Ok(None)` silently discarded every one of them. The 2 `ast_grep_search` events that did get through recorded `"unknown"` because the adapter read the wrong key.

So the search signal was never excluded by the observer-first Bash policy. It was **thrown away by the adapter**. Bash capture is irrelevant to this gap.

**Decision: no Bash capture change.** The observer-first default stands unchanged. The search gap is closed by §3 of this change — mapping `grep`, `find`, and `ast_grep_search` to `SearchRun` on their real `pattern` key. Claude Code's `Grep`, `Glob`, and `WebSearch` were already mapped and already correct.

Consequences:
- `vision.md`'s "repeated search terms" claim needs **no downward correction**. It becomes supportable for the first time, rather than being withdrawn. Correcting it downward would have been the wrong fix for a misdiagnosed cause.
- `roadmap.mdx` needs no Phase 1/Phase 8 change on this account.
- The three options previously listed here are moot: option 1 (capture Bash) and option 2 (parse search commands out of Bash) both solve a problem that does not exist, and option 3 (withdraw the claim) would have documented a defect as a limitation.

**The `FileOpened` co-occurrence thread survives on its own merits.** There are 386 `FileOpened` events across 30 sessions, and which files get opened *together within a session* is a real, fully-captured, entirely unmined signal. It directly answers "what should I load alongside this?" — which is what `route bundle` needs to emit — and it does not depend on search terms at all. It is recorded as a named future thread, not as a fallback for a gap that no longer exists.

**Lesson worth keeping:** the original D7 reasoned from repository instructions to conclude a product claim was unsupportable, and would have led to weakening `vision.md` to match a bug. Verifying the premise against the corpus took one query. A "deliberate trade-off" that nobody measured is indistinguishable from a defect nobody found.

## Implementation deviations (recorded 2026-07-25)

### A1. The unconditional diagnostic channel is the warnings log, not stderr

D3 required the missing-field diagnostic to be "unconditional, not gated on `SCRYRS_DEBUG`," and named stderr. Implementation found stderr is the wrong channel.

Claude Code's hook documentation states that for `PostToolUse`, stderr can be surfaced ("Shows stderr to Claude; the tool already ran"). Writing unconditionally to stderr from a hook therefore risks putting scryrs diagnostics into the agent-visible transcript — a non-interference violation, and precisely the kind of "modifies what the agent sees" behavior `trace-hook-contract` forbids.

The existing `warn()` in `crates/scryrs-cli/src/hook.rs` already provides the right shape: it appends unconditionally to `.scryrs/hooks/<harness>-warnings.log` and writes to stderr **only** under `SCRYRS_DEBUG`. That satisfies D3's actual intent — the diagnostic is always recorded and always findable — without risking transcript pollution.

**Decision:** the unconditional channel is the warnings log. Stderr stays debug-gated. D3's requirement is met by the log, not by stderr.

Mechanically, a missing key field returns `Err(AdapterError::MissingField { .. })` rather than `Ok(None)`. The hook already treats every `AdapterError` as fail-open — log the warning, persist nothing, exit 0 — so this reuses the existing path and keeps `Ok(None)` meaning exactly one thing: "untracked tool, pass-through."

### A2. External paths are marked via a content-dependent `subject_kind`

D2 deferred the external-path marker mechanism to task work, requiring no `SCHEMA_VERSION` bump if avoidable. It is avoidable.

`TraceEvent::subject_kind()` is a **derived query column**, not part of the serialized payload that `SCHEMA_VERSION` governs. Making it content-dependent for path-bearing payloads costs no schema change:

- normalized (repository-relative, therefore never absolute) → `"file"`
- external (recorded verbatim, therefore absolute) → `"external_file"`

Absoluteness is a sound discriminator precisely *because* normalization guarantees internal paths are relative. The two cases cannot collide.

Bonus correctness: `scryrs-runtime::default_load_target` already falls through to `RouteLoadTargetKind::NonLoadable` for unrecognized subject kinds, so external files become non-loadable route targets with **no runtime change required** — which is exactly right, since a path outside the repository is not loadable context for a reader of this repository.

`HOTSPOT_SCHEMA_VERSION` is not bumped: `subjectKind` remains a string field and gains an additional possible value, which is additive for consumers.

### A3. FailedLookup's subject_kind becomes path-based, and SymbolInspected goes dead

`subject_kind()` currently maps `FailedLookup` to `"symbol"`, alongside `SymbolInspected`. That was correct when `lsp_navigation` was `FailedLookup`'s only source. After D5 (failed reads become `FailedLookup`) and the D4 decision to drop `lsp_navigation`, **every** `FailedLookup` carries a file path.

Leaving it as `"symbol"` would be actively harmful: a failed read of `crates/x.rs` would be grouped under `("symbol", "crates/x.rs")`, separate from that same file's successful reads under `("file", "crates/x.rs")` — re-introducing exactly the split-subject defect this change exists to remove.

**Decision:** `FailedLookup` derives its `subject_kind` from its path like `FileOpened` and `EditMade` do (`"file"` / `"external_file"`). A file's successful and failed accesses group into one entry, so `counts.eventType` carries both `FileOpened` and `FailedLookup` for the same subject. This is what makes `memory_patch`'s failure ratio and `debugging_playbook`'s `FailedLookup` count meaningful on a real corpus.

**Consequence to flag, not fix here:** with `lsp_navigation` dropped, `SymbolInspected` has no producer on either harness — Claude Code never emitted it, and it was Pi-only via an extension tool. The `"symbol"` subject kind therefore becomes unreachable. Under AGENTS.md Rule 9 this is dead code, but removing `TraceEventType::SymbolInspected` touches the serialized payload enum and would require a `SCHEMA_VERSION` bump. That is out of scope for this change and is raised as separate follow-up work rather than absorbed here.

### A4. Historical migration: decided, and two near-misses worth recording

Task 4.6's choice is **migrate in place**. Implemented as a one-time data migration on `EventStore::open`, guarded by a `subject_normalization_migrated` marker in `schema_meta`. The table shape is unchanged, so `datastore_schema_version` is untouched.

Verified on the dogfooded store (backed up first):

| | before | after |
|---|---|---|
| path rows | 501 | 501 |
| absolute subjects | 166 | 23 (all genuinely external) |
| distinct subjects | 213 | 183 |
| **files double-counted** | **30** | **0** |
| hotspot entries | 215 | 185 |

`.devagent/docs/docs/roadmap.mdx` went from score 20 to 28 once its two spellings merged — a concrete example of the split this change existed to remove.

**Two bugs were caught by running it for real, both of which would have silently corrupted a user's store:**

1. **Wrong root from a container mount.** The first run happened inside the verification container, where the repository is bind-mounted at `/workspace`. The derived root was `/workspace`, so all 166 internal absolute subjects were classified `external_file` and the store was marked migrated — permanent damage on the next run. Fix: `root_matches_recorded_subjects` requires the derived root to be a prefix of at least one recorded absolute subject; otherwise skip **without** marking, so a later open under the correct root still migrates.

2. **Empty root from a relative store path.** `record --mode local` opens the *relative* `CANONICAL_STORE_PATH` (`.scryrs/scryrs.db`), so `parent().parent()` yielded the empty path. The prefix guard then built `"/"`, which matches every absolute subject, so the guard passed and the migration again marked everything external. Fix: resolve the store path with `canonicalize()` before deriving the root, and reject a root that is empty or non-absolute.

Both were invisible to unit tests, which construct stores at absolute temp paths. Both are now covered: `migration_skips_a_root_that_does_not_match_recorded_subjects` and `migration_resolves_a_relative_canonical_store_path`.

**Accepted limitation:** a store containing only *external* absolute subjects and no internal ones cannot prove its root, so it is skipped and those rows keep `subject_kind = "file"`. This is cosmetic — absolute and relative subject strings cannot collide, so nothing is falsely merged — and it is strictly preferable to the alternative of guessing and corrupting.

### A5. The reclassification is forward-only, so `debugging_playbook` stays unreachable on historical data

Reachability measured against the migrated store:

| proposal kind | gate | eligible entries |
|---|---|---|
| `skill` | failure outcome > 0 | 13 |
| `memory_patch` | failure ratio ≥ 0.5 and score ≥ 4 | 3 |
| `debugging_playbook` | `FailedLookup` ≥ 2 | **0** |

`debugging_playbook` is still unreachable on the existing corpus, and this change does not make it reachable retroactively. The reason: historical rows recorded a failed read as `FileOpened` with `Outcome::Failure`, and the migration normalizes **subjects only** — it does not rewrite `event_type`. The 13 failure events that should be `FailedLookup` remain filed as failed `FileOpened`.

This is a deliberate scope boundary, not an oversight. The approved migration was subject normalization; retro-classifying event types is a different and heavier rewrite (it changes the event family, the payload variant, and therefore what every downstream consumer sees for those rows), and it was not the decision that was taken.

Consequence to state plainly: **`debugging_playbook` becomes reachable only from newly-recorded events.** Every failed read captured after this change lands as `FailedLookup`, so the gate will start firing as new evidence accumulates — but task 5.6 cannot be demonstrated against today's store. A follow-up retro-classification migration is the option if that wait is unacceptable.

## Risks / Trade-offs

- **Hotspot rankings change.** Normalization collapses 30 double-counted files and re-ranks. Snapshot tests will break; regenerate them deliberately per `testing-this-repo`, and inspect the diff rather than accepting it blind.
- **`PostToolUse` coverage on tool crashes is unverified.** D6 depends on it. Verify against a real session before committing to the migration; if coverage is worse than expected, the honest outcome is a documented partial.
- **Existing stores hold un-normalized rows.** Migration or a documented cut-line, not both, and not silence.
- **Broadening `FailedLookup` changes proposal-generation behavior** without touching curator thresholds. The curator is downstream and its tests may shift; that is expected and must be reviewed, not suppressed.
- **Fixing the signal invalidates the current `.scryrs/hotspots.json`** and any conclusions drawn from it.

## Migration Plan

1. Verify Pi field names and `PostToolUse` behavior empirically. Nothing else starts until these two facts are known.
2. Shared extraction helper with drop-and-diagnose, replacing both silent defaults.
3. Subject normalization plus external-path marking.
4. `FailedLookup` reclassification across both adapters.
5. Claude Code `PostToolUse` migration and `init` registration replacement.
6. Regenerate snapshots; confirm curator reachability for each proposal kind against the real store.
7. Resolve D7 and correct `vision.md` / `roadmap.mdx` to match.

## Open Questions

- What are Pi's actual `input` key names for `ast_grep_search` and `lsp_navigation`? (Blocks §3; resolved by task 1.1.)
- Does Claude Code fire `PostToolUse` when a tool crashes rather than returning an error? (Blocks D6; resolved by task 1.2.)
- Does the external-path marker fit without a `SCHEMA_VERSION` bump?
- After D5, does `memory_patch` still have a reachable trigger, or does it need re-gating on `FailedLookup`?
- Which D7 option, and does `vision.md` keep or lose the "repeated search terms" claim?
