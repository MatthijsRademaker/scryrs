## Why

scryrs needs to establish itself as observer-first hotspot detector before it grows runtime optimization ambitions. Bash command capture is currently noisy, low-signal, and fragments hotspot subjects, so leaving it enabled by default weakens product story instead of strengthening it.

## What Changes

- **BREAKING**: Stop recording Bash/CommandExecuted events by default in both reference harness integrations.
- Re-enable Bash observation only when explicit debug environment variable is set, so command tracing becomes opt-in diagnostic path instead of default product behavior.
- Tighten reference-hook docs, manifest metadata, and verification coverage around “native observer tools first” boundary.
- Keep hotspot scoring focused on stable observer-native signals such as file reads, edits, symbol inspection, and search/navigation activity.
- Add thin roadmap documentation for future rewrite/optimizer direction modeled on RTK-style command rewriting, while keeping that work explicitly out of current implementation scope.

## Capabilities

### New Capabilities

- `observer-first-roadmap`: Document future runtime-optimizer/rewrite direction as roadmap-only follow-up work, without changing current execution semantics.

### Modified Capabilities

- `pi-reference-hook`: Change default Pi trace capture so Bash is excluded unless explicit debug mode is enabled.
- `claude-code-reference-hook`: Change default Claude Code trace capture so Bash is excluded unless explicit debug mode is enabled.
- `cross-harness-verification`: Update verification fixtures to assert no default Bash capture and debug-gated Bash capture for both harnesses.
- `scryrs-manifest`: Update harness metadata so default observed-tool and captured-family declarations reflect debug-gated Bash capture.
- `trace-hook-contract`: Clarify observer-first product boundary, default non-capture of Bash, and future rewrite/optimizer work as roadmap-only.

## Impact

- Affected code: `hooks/pi/`, `hooks/claude-code/`, verification scripts/fixtures, `scryrs.json`, and project docs under `.devagent/docs/docs/` plus hook READMEs.
- Behavioral impact: default trace data becomes less noisy and more aligned with hotspot detection on stable native tool signals.
- Product impact: scryrs positioning becomes cleaner — observer first now, rewrites later.
