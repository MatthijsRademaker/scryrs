## Context

The Pi trace hook is intentionally transport-only and fail-open. It subscribes to `session_start` and `tool_result`, maps tracked Pi tools into canonical `TraceEvent` JSONL, then delegates ingestion to `scryrs record --stdin` through `pi.exec`. That boundary protects agent tool execution, but it also hides the exact failure point when trace capture stops working.

The current verification lane proves persistence by checking `.scryrs/scryrs.db`, but it does not expose intermediate evidence such as whether the hook loaded, which tool names Pi emitted, which input fields were present, whether a tracked tool was skipped, or why `scryrs record` rejected a line. The Pi hook also relies on assumed field names (`event.input.query`, `event.input.symbol`, `event.input.command`, `event.input.path`) that need empirical verification against real Pi runtime events and rewrite extensions.

The repository has one canonical Pi hook source (`hooks/pi/index.ts`) and one ignored installed runtime copy (`.pi/extensions/pi-trace/index.ts`). This change must preserve that ownership: edit canonical source only, then refresh installed runtime copy through `scryrs init --agent pi`.

## Goals / Non-Goals

**Goals:**

- Provide opt-in developer debug logging for each meaningful stage of the Pi trace pipeline.
- Provide opt-in record-side debug logging for input receipt, validation, rejection, datastore persistence, and summary.
- Keep default behavior byte-compatible enough that existing CLI stdout/stderr tests and hook non-interference tests remain valid when `SCRYRS_DEBUG` is unset.
- Echo `scryrs record` debug stderr/stdout through the hook debug path so Pi developers can see record-side breadcrumbs from the Pi process.
- Make debug output structured enough for verification scripts to assert key pipeline stages.
- Avoid leaking full tool outputs, file contents, or large payloads in default debug mode.

**Non-Goals:**

- No change to TraceEvent schema, datastore schema, scoring, dashboard APIs, or hook event mappings beyond logging.
- No change to fail-open behavior: hook failures still must not block or mutate tool results.
- No new logging framework or external dependency.
- No automatic command canonicalization, RTK rewrite normalization, or field-name remapping.
- No direct edits to `.pi/extensions/pi-trace/index.ts`.
- No global always-on logs in normal operation.

## Decisions

### D1: `SCRYRS_DEBUG` gates all new debug output

Chosen: enable debug logs only when `SCRYRS_DEBUG` is set to a non-empty value.

Rationale: developers need a no-rebuild toggle, while normal users and existing tests need current quiet behavior. The hook and CLI can both read the same environment variable without adding config files or flags.

Rejected alternatives:
- Always log failures: rejected because it increases normal stderr noise and can break tests or user expectations.
- Add CLI flags only: rejected because the Pi hook invokes `scryrs record --stdin` internally and needs environment-driven visibility without changing the hook subprocess contract.

### D2: Use structured single-line text logs, not JSON-only logs

Chosen: emit one physical line per debug breadcrumb with stable prefixes such as `[scryrs]` for hook logs and `[scryrs-record]` for record logs, followed by key/value fields.

Rationale: line-oriented logs are easy for humans to read and easy for scripts to assert. They avoid colliding with `scryrs record`'s normal stdout JSON summary and stderr JSONL rejection diagnostics because they only appear when explicitly enabled.

Rejected alternative:
- Pure JSON logs: rejected for this pass because normal `record` stderr already uses JSONL for rejection diagnostics, and mixing debug JSON with rejection JSON would require consumers to distinguish two JSON object families.

### D3: Define debug levels with safe default inspection

Chosen:
- `SCRYRS_DEBUG=1`: breadcrumbs, input key lists, event types, tool names, subprocess result codes, truncated child stdout/stderr previews.
- `SCRYRS_DEBUG=wire`: level 1 plus sanitized input previews for known fields.
- `SCRYRS_DEBUG=raw`: capped raw envelope/event previews for local troubleshooting only.

Rationale: most debugging needs keys and stage transitions, not full event bodies. Wire/raw modes provide deeper inspection when field names or RTK mutation propagation must be verified.

Rejected alternative:
- Single raw dump mode: rejected because `write` inputs and tool results may include file contents or secrets.

### D4: The hook logs untracked tools in debug mode

Chosen: when debug is enabled, every `tool_result` logs `tool=<name>`, `tracked=<true|false>`, `is_error=<bool>`, and `input_keys=<...>` before returning.

Rationale: the most likely Pi integration failure is a tool-name or field-name mismatch. Untracked logs reveal whether Pi emits names such as `exec_command` instead of `bash`, or whether custom tool namespaces differ from the current hook assumptions.

Rejected alternative:
- Only log tracked tools: rejected because it misses the most useful diagnostic for skipped events.

### D5: Field fallback logs include available keys

Chosen: when a mapped field is missing, log the wanted field, available input keys, and fallback value before emitting `unknown`.

Rationale: current fallback warnings say a field is missing but do not show what the hook actually observed. Available-key logging makes field drift obvious without requiring raw dumps.

Rejected alternative:
- Fail or skip events with missing fields: rejected because it would change existing fail-open/defensive behavior.

### D6: Hook debug echoes child process streams

Chosen: when debug is enabled, `recordEvent` logs subprocess code, killed status, and truncated previews of stdout and stderr for every `scryrs record --stdin` invocation.

Rationale: record-side debug logs written to stderr are otherwise trapped inside `pi.exec`. Echoing them through the hook gives one visible pipeline trace in Pi logs.

Rejected alternative:
- Only log non-zero exits: rejected because successful record-side breadcrumbs are needed to verify end-to-end operation.

### D7: Record-side debug does not change normal contracts when disabled

Chosen: `crates/scryrs-cli/src/record.rs` writes debug lines to stderr only when `SCRYRS_DEBUG` is set. Normal stdout summary, stderr rejection JSONL, and exit codes remain unchanged with debug disabled.

Rationale: existing tests and integrations depend on deterministic record output. Debug is a developer mode, not a new public data channel.

Rejected alternative:
- Add debug fields to the stdout summary: rejected because it changes machine-readable command output.

### D8: Verification asserts breadcrumbs, not full log order

Chosen: tests should assert presence of key debug stages and stable fields, not strict full ordering.

Rationale: `session_start` is fire-and-forget and may race with tool event handling. Presence checks catch missing stages without creating flaky ordering constraints.

Rejected alternative:
- Assert exact complete log output: rejected because timestamps, UUIDs, child stream details, and scheduling can vary.

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| Debug logs leak sensitive tool input. | Default debug logs keys and summaries only; wire/raw modes are explicit and capped. Do not dump tool `content` or `details` by default. |
| Debug stderr breaks existing tests. | Gate all new output behind `SCRYRS_DEBUG`; update/add tests for both disabled and enabled modes. |
| Record debug lines mix with rejection diagnostics. | Keep debug prefixed as `[scryrs-record]` and only emit when explicitly enabled. Existing JSONL consumers remain unaffected by default. |
| Hook logs become noisy in real Pi sessions. | Make logs opt-in and line-oriented; document targeted debugging workflow. |
| Verification becomes flaky due to lifecycle races. | Assert stage presence and counts, not exact order. |
| Installed Pi hook drifts from canonical source. | Refresh installed artifact through `scryrs init --agent pi` after canonical edit; never patch `.pi/extensions/pi-trace/index.ts` directly. |

## Migration Plan

1. Add hook-side debug helpers and stage logs in `hooks/pi/index.ts`.
2. Add record-side debug helpers in `crates/scryrs-cli/src/record.rs`.
3. Extend verification scripts to exercise `SCRYRS_DEBUG=1` and assert key breadcrumbs.
4. Document debug usage and safety in Pi hook and/or verification docs.
5. Refresh `.pi/extensions/pi-trace/index.ts` by removing the installed artifact and running `scryrs init --agent pi`.
6. Rollback by unsetting `SCRYRS_DEBUG` for immediate quiet behavior; code rollback is safe because no schema or persistence migration is involved.

## Open Questions

- Should `SCRYRS_DEBUG=wire` include sanitized known field values for all tools, or only key lists plus specific fields needed to diagnose current Pi mappings?
- Should `SCRYRS_DEBUG=raw` be documented publicly, or kept as an implementation detail to avoid encouraging unsafe dumps?
- Should Claude Code hook receive the same debug-mode treatment later for consistency, or stay out of scope until a separate need appears?
