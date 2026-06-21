## Why

`Hotspot Foundation 03` is not a greenfield feature anymore. Repository inspection and archived `Hotspot Foundation 02` artifacts show that the SQLite read path, scoring engine, report types, and CLI wiring already exist. The open task remains because the shipped `scryrs hotspots <PATH>` surface still has contract gaps and stale placeholder language that prevent calling the command a finished standalone product capability.

The remaining work is to make the existing implementation match the canonical `hotspot-report` contract exactly: deterministic metadata, deterministic final tie-break behavior, honest artifact-write failure handling, and user-facing text that no longer claims hotspot output is a placeholder.

## What Changes

1. **Close the remaining hotspot contract gaps in the live CLI path**
   - Compute `runMetadata.firstEventId` and `runMetadata.lastEventId` as the minimum and maximum SQLite row ids among subject-bearing events.
   - Make the ranking tie-break use the SQLite id of each subject's chronologically first contributing event, matching evidence order rather than the minimum row id in the group.
   - Treat `.scryrs/hotspots.json` write failures as command failures instead of reporting success.

2. **Remove stale placeholder-only hotspot language from user-visible surfaces**
   - Update `scryrs --help` hotspot wording.
   - Update `--help-json`/snapshot expectations to reflect the real `HotspotsReport` surface.
   - Update README hotspot examples and descriptions so they describe SQLite-backed analysis rather than placeholder output.

3. **Add targeted repeatable tests for the edge cases that prove the contract**
   - Lifecycle-only stores return exit 0 with `entries: []`.
   - Non-monotonic timestamp/id cases preserve deterministic ordering and correct `firstEventId` semantics.
   - Evidence row ids stay in chronological `timestamp ASC, id ASC` order.
   - Successful runs write artifact contents byte-for-byte equal to stdout.
   - Artifact write failures return the contracted error behavior.
   - Help/snapshot coverage proves placeholder wording is gone.

## Impact

- **Code paths touched**: `crates/scryrs-cli`, `crates/scryrs-core`, related CLI tests/snapshots, and `README.md`.
- **Behavioral change**: artifact write failure stops being a logged-but-successful run and becomes a failing invocation.
- **No new subsystem scope**: no graph, proposal, adapter, runtime, dashboard, or LLM work is added.
- **No datastore redesign**: the command continues to read `<PATH>/.scryrs/scryrs.db` through the existing read-only query path and uses the existing hotspot report schema/versioning contract.