## 1. Extend live bootstrap configuration inputs

- [x] 1.1 Add a live-bootstrap Docker network input to CLI parsing, help text, and help-json for `scryrs init`.
- [x] 1.2 Extend live config resolution so the Docker network value resolves deterministically from flags, environment, and `.scryrs/.env` before any live init writes.
- [x] 1.3 Add validation/error coverage proving live init fails with exit 2 and no partial writes when required bootstrap inputs are missing or conflicting.

## 2. Scaffold managed workspace live bootstrap files

- [x] 2.1 Implement live-mode `.scryrs/compose.yml` generation for consumer workspaces with external-network attachment, `scryrs` endpoint contract, and persistent server storage.
- [x] 2.2 Update live-mode `.scryrs/.env` scaffolding so it persists live ingest identity plus Docker network configuration without clobbering equivalent managed values.
- [x] 2.3 Keep `scryrs.json` live-mode writes limited to the `remote` section while preserving unrelated manifest keys and skipping `.scryrs/scryrs.db` creation.
- [x] 2.4 Add tests covering first live init, second-harness live init reuse, and conflicting managed bootstrap values.

## 3. Make harness installation idempotent with shared live bootstrap

- [x] 3.1 Change Pi file-install behavior so byte-identical existing content is accepted as a no-op and divergent content still fails loudly.
- [x] 3.2 Preserve existing Claude Code idempotency while ensuring second-harness init does not rewrite shared live bootstrap artifacts.
- [x] 3.3 Update init next-step text so live-mode success points to `scryrs up` and no longer tells users to use the scryrs repo-root Compose workflow.

## 4. Add `scryrs up` as thin compose orchestration

- [x] 4.1 Add `scryrs up` to dispatch, help text, and help-json with a narrow contract: start the workspace-managed Compose stack only.
- [x] 4.2 Implement `scryrs up` validation for required scaffold files and deterministic Docker/Compose invocation against `.scryrs/compose.yml` and `.scryrs/.env`.
- [x] 4.3 Add failure handling and tests for missing scaffold files, missing external network, and unexpected arguments.

## 5. Update packaging/docs and verify end to end

- [x] 5.1 Update root Docker/Compose smoke expectations so repository artifacts remain packaging/dev assets while consumer bootstrap is generated under `.scryrs/`.
- [x] 5.2 Rewrite `live-server-setup.md`, `cli-v0-contract.md`, and related live-mode docs to make the workspace-local bootstrap flow and external-network `http://scryrs:8081` contract explicit.
- [x] 5.3 Run targeted CLI tests, smoke checks, and docs verification to prove the new bootstrap flow, idempotency, and documentation all match the intended contract.
