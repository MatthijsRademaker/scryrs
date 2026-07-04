## Context

The Pi hook has one canonical source file at `hooks/pi/index.ts` and one installed runtime copy created by `scryrs init --agent pi`. Today that installed copy lives at `.pi/extensions/pi-trace/index.ts`, and the same name appears in the installer, doctor checks, verification fixtures, ignore rules, maintainer guidance, and Pi hook README. The `pi-trace` label is stale branding. Because this surface is still alpha, the repository does not need migration compatibility, dual-path detection, or legacy warning behavior.

## Goals / Non-Goals

**Goals:**

- Rename the installed Pi extension directory from `pi-trace` to `scryrs` everywhere that defines or verifies the supported install path.
- Keep canonical-source ownership explicit: `hooks/pi/index.ts` remains source of truth and `.pi/extensions/scryrs/index.ts` remains runtime copy only.
- Keep harness identifiers stable so the change stays localized: `--agent pi` and `scryrs hook pi` do not change.
- Remove all product-surface references that still describe supported Pi installation through `pi-trace`.
- Make verification and docs enforce hard-cut behavior with no legacy fallback.

**Non-Goals:**

- No rename of harness IDs, subcommands, adapter names, or Rust module names from `pi` to `scryrs`.
- No support for both `.pi/extensions/pi-trace/` and `.pi/extensions/scryrs/` at the same time.
- No doctor migration hint, compatibility probe, or automatic cleanup of old `pi-trace` directories.
- No changes to Claude Code runtime behavior beyond incidental doc wording where Pi and Claude are described together.

## Decisions

### 1. Installed Pi extension path hard-cuts to `.pi/extensions/scryrs/index.ts`

`scryrs init --agent pi` will write only `.pi/extensions/scryrs/index.ts` for consumer installs and source-repo dogfooding. The old `.pi/extensions/pi-trace/index.ts` path becomes unsupported immediately.

**Why:** This removes stale naming at the only supported install path and avoids carrying alpha-era path debt into later releases.

**Alternatives considered:**

- Keep `pi-trace` and only reword docs: rejected because it preserves wrong product-facing path.
- Add dual-path support: rejected because it adds needless complexity and violates the requested hard cut.

### 2. Harness identity stays `pi`

CLI surfaces and hook dispatch remain `--agent pi` and `scryrs hook pi`. Only installed directory naming changes.

**Why:** Path branding fix is small and surgical. Renaming harness IDs would expand scope into CLI help, adapters, command routing, and external user workflow without solving the actual naming problem.

**Alternatives considered:**

- Rename harness to `scryrs`: rejected because it is a wider breaking API change than needed.

### 3. Verification and docs use only supported path

Installed-hook verification, maintainer docs, ignore rules, and Pi hook README will refer only to `.pi/extensions/scryrs/` and `~/.pi/agent/extensions/scryrs/` as the supported installed locations.

**Why:** Hard-cut naming only works if every repository-owned source of truth converges on one path.

**Alternatives considered:**

- Mention both old and new paths during transition: rejected because it weakens the hard cut and leaves ambiguity in support status.

### 4. Doctor path detection changes with no legacy handling

`scryrs doctor` will check only `.pi/extensions/scryrs/index.ts` when reporting Pi hook presence. An old `pi-trace` install will be treated the same as any other missing Pi hook.

**Why:** Doctor is diagnostic surface, not migration assistant. In alpha, clean supported-path detection is better than preserving legacy recognition logic.

**Alternatives considered:**

- Detect both paths and warn about legacy install: rejected because user explicitly does not want backward-compatibility behavior, including in doctor.

### 5. Canonical-source split remains explicit

Repository guidance will continue to distinguish `hooks/pi/index.ts` as canonical source from installed runtime copy under `.pi/extensions/scryrs/index.ts`.

**Why:** Path rename does not change ownership risk. Two editable copies would still create drift.

**Alternatives considered:**

- Treat installed copy as co-equal source: rejected because it preserves same authorship ambiguity under a new path.

## Risks / Trade-offs

- Existing local `pi-trace` installs stop working as supported artifacts → Accept breakage; users rerun `scryrs init --agent pi`.
- Missed string replacement in tests, docs, or specs leaves repository inconsistent → Update installer, doctor, verification, docs, and living specs in same change.
- Pi runtime could rely on extension-directory naming in a way not captured by current tests → Keep installed-hook e2e and explicit Pi reload guidance aligned with new directory; verify real installed artifact path in tests.
- Removing old ignore rule could surface stale local artifacts in git status if present → Acceptable in alpha; supported ignored path becomes `.pi/extensions/scryrs/` only.

## Migration Plan

1. Update installer constants and next-step text to emit `.pi/extensions/scryrs/index.ts`.
2. Update doctor, tests, verification fixtures, ignore rules, and repository guidance to use `scryrs` path only.
3. Update live OpenSpec specs that encode the old path.
4. Users with local `pi-trace` installs rerun `scryrs init --agent pi`; no automatic migration or compatibility layer is provided.

## Open Questions

- None. Hard-cut path rename, no compatibility handling, and stable harness IDs are already decided.
