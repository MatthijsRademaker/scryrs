## ADDED Requirements

### Requirement: Workspace live bootstrap is scaffolded under `.scryrs/`

Live-mode consumer bootstrap SHALL be workspace-local. When `scryrs init --agent <name>` succeeds in live mode, the installer SHALL scaffold managed runtime files under the target project's `.scryrs/` directory instead of requiring the operator to check out the scryrs source repository and run the repo-root Compose stack there.

#### Scenario: Live init creates managed workspace bootstrap files

- **WHEN** `scryrs init --agent pi` or `scryrs init --agent claude-code` succeeds in a consumer project in live mode
- **THEN** `.scryrs/compose.yml` and `.scryrs/.env` exist under that project
- **AND** the workspace-local scaffold is sufficient input for later `scryrs up`
- **AND** the operator is not required to copy or invoke the scryrs repository's root `docker-compose.yml`

#### Scenario: Second harness install reuses workspace bootstrap

- **GIVEN** a consumer project already has a successful live bootstrap scaffold under `.scryrs/`
- **WHEN** `scryrs init --agent <other-supported-harness>` is run with equivalent live bootstrap values
- **THEN** the existing managed `.scryrs/compose.yml` and `.scryrs/.env` are preserved
- **AND** only the newly requested harness installation work is applied

### Requirement: Scaffolded compose joins an existing external agent network as `scryrs`

The managed `.scryrs/compose.yml` SHALL attach the live server container to a configured external Docker network and make the server reachable to peer agent containers there as `http://scryrs:8081`. The scaffold SHALL not require agent containers to join a scryrs-owned dedicated network.

#### Scenario: Peer containers resolve the live server via external network alias

- **GIVEN** the configured external Docker network exists and both the scaffolded scryrs service and an agent container are attached to it
- **WHEN** the agent resolves the configured live ingest URL
- **THEN** it can reach the live server at `http://scryrs:8081`
- **AND** the reachability contract depends on the shared external network attachment rather than a host-local loopback address

#### Scenario: Scaffold uses external network attachment instead of dedicated scryrs network

- **WHEN** live bootstrap scaffolds `.scryrs/compose.yml`
- **THEN** the compose file declares an external Docker network input
- **AND** the scryrs service joins that network with the `scryrs` endpoint contract
- **AND** the scaffold does not create or require a dedicated `scryrs-net` network for consumer deployments

### Requirement: `scryrs up` is a thin workspace-local compose launcher

The `scryrs up` command SHALL start the live server from the workspace-managed `.scryrs/compose.yml` and `.scryrs/.env` only. It SHALL not install hooks, infer or rewrite runtime identity, or mutate unrelated workspace state.

#### Scenario: `scryrs up` launches the managed compose stack

- **GIVEN** a consumer project contains a valid `.scryrs/compose.yml` and `.scryrs/.env`
- **WHEN** `scryrs up` is invoked from that project
- **THEN** the command launches the compose stack defined by those files
- **AND** it uses the workspace-managed bootstrap files rather than the scryrs repository root artifacts

#### Scenario: `scryrs up` fails loudly when scaffold prerequisites are missing

- **WHEN** `scryrs up` is invoked in a project without the required managed compose scaffold or configured external network
- **THEN** the command exits non-zero with deterministic remediation guidance
- **AND** it does not attempt unrelated hook installation or live-config mutation
