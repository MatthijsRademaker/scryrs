## Context

`scryrs init` currently enforces a blanket self-install refusal for any invocation inside the scryrs source checkout. That rule protected a clean source/consumer boundary when the installer was introduced, but it also blocks the most direct Pi dogfooding workflow inside this repository. At same time, root-repo Pi install has real sharp edges if done carelessly: nested installs from subdirectories, stale duplicate hook copies, accidental edits against installed artifact instead of canonical source, and noisy git status from local runtime files.

This change therefore needs narrower behavior than “remove self-install guard.” It must preserve source-of-truth discipline while allowing a deliberate repo-local Pi install for testing.

## Goals / Non-Goals

**Goals:**
- Allow `scryrs init --agent pi` to succeed inside scryrs source checkout for local dogfooding.
- Keep `scryrs init --agent claude-code` blocked inside source checkout.
- Preserve `hooks/pi/index.ts` as only canonical Pi hook source in repository.
- Prevent nested `.pi/extensions/pi-trace/` installs when command is run from repo subdirectories.
- Keep installed root-repo Pi artifact out of normal git noise and make maintainer/agent ownership explicit.

**Non-Goals:**
- No change to Claude Code root-repo install policy.
- No new `--target-dir`, `--force`, or overwrite behavior.
- No change to Pi hook business logic, event mappings, or runtime contract.
- No commitment that `.pi/extensions/pi-trace/index.ts` becomes checked-in source.

## Decisions

### D1: Replace blanket self-install refusal with harness-specific source-repo policy

Chosen:
- If source checkout is detected and `--agent claude-code`, installer still exits 2.
- If source checkout is detected and `--agent pi`, installer is allowed to continue.

Rejected alternatives:
- Allow both harnesses in source repo: rejected because Claude Code still writes consumer `.claude/` config into source repo with no comparable project-scoped justification.
- Keep blanket refusal and add separate dev-only script: rejected because it preserves friction and duplicates installer semantics instead of fixing source of pain.

Rationale: Pi already has project-scoped `.pi/` configuration in this repository. Allowing Pi only is smallest policy change that unlocks dogfooding without discarding original boundary entirely.

### D2: Source-repo Pi installs resolve to checkout root, not caller CWD

Chosen:
- When installer detects scryrs source checkout and `--agent pi`, target root becomes detected checkout root.
- `.pi/extensions/pi-trace/index.ts` is therefore always created under repository root even if command is launched from a nested directory.

Rejected alternative:
- Continue writing relative to current directory: rejected because it would create nested `.pi/` trees (`crates/.../.pi/extensions/pi-trace`) and make root-repo dogfooding unreliable.

Rationale: dogfooding intent is “enable Pi trace hook for this repository,” not “create arbitrary subproject-local copies.”

### D3: Canonical-source split is explicit and normative for maintainers/agents

Chosen:
- `hooks/pi/index.ts` remains canonical source.
- `.pi/extensions/pi-trace/index.ts` is installed runtime copy only.
- `AGENTS.md` explicitly states LLMs/agents must not treat installed copy as leading source and must not edit it directly.

Rejected alternative:
- Treat installed copy as co-equal source: rejected because it creates two editable copies of same hook and guarantees drift.

Rationale: once root-repo install is allowed, ambiguity becomes main risk. Explicit source ownership must become written contract, not tribal knowledge.

### D4: Local installed Pi artifact is ignored rather than committed

Chosen:
- Repository ignore rules cover `.pi/extensions/pi-trace/` so local dogfooding does not dirty normal working tree views.

Rejected alternative:
- Commit installed artifact: rejected because repository would then carry canonical source plus generated install copy.

Rationale: installed copy exists for runtime convenience, not authorship.

### D5: Existing collision contract stays intact

Chosen:
- If `.pi/extensions/pi-trace/index.ts` already exists, `scryrs init --agent pi` still exits 2 with collision guidance.

Rejected alternative:
- Auto-overwrite installed copy in source repo: rejected for this change because it would silently mutate local runtime config and cut across established init semantics.

Rationale: smallest safe change. Refresh workflow can remain remove-and-rerun unless future evidence justifies explicit overwrite mode.

## Risks / Trade-offs

- **Installed copy drifts from canonical source after hook edits** → Mitigate with AGENTS guidance that `hooks/pi/index.ts` is sole edit target and installed copy must be refreshed by reinstall, never patched manually.
- **Subdirectory invocation targets wrong path** → Mitigate by resolving source-repo Pi install to checkout root.
- **Developers mistake ignored installed copy for source of truth** → Mitigate with explicit AGENTS entry and, if touched, README wording.
- **Policy asymmetry between Pi and Claude Code feels surprising** → Mitigate with docs/spec text that explains why Pi is project-scoped in this repo while Claude Code remains consumer-only.
- **Ignored path can hide accidental debugging edits** → Accept trade-off; canonical-source rule and collision behavior limit casual mutation, and installed copy is intentionally non-canonical.

## Migration Plan

1. Relax self-install logic only for `--agent pi`.
2. Resolve source-checkout root and install Pi artifact there.
3. Update/extend init tests for allowed root-repo Pi install, continued Claude Code refusal, and subdirectory root resolution.
4. Add repository guidance clarifying canonical-vs-installed Pi paths and LLM editing prohibition.
5. Ignore installed Pi artifact path so local dogfooding stays low-noise.

## Open Questions

- Does AGENTS guidance alone suffice, or should a project-local rule file under `.pi/rules/` also restate “never edit installed copy”? Proposal assumes AGENTS.md is minimum required contract and implementation can decide if extra rule file is warranted.
