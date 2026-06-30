## MODIFIED Requirements

### Requirement: Compose example supports persistent multi-agent networking

The repository SHALL provide a live-server compose definition that supports persistent SQLite storage and lets the scryrs server join an existing external agent network under the `scryrs` endpoint contract. Consumer-facing live bootstrap SHALL not require agent containers to join a scryrs-owned dedicated network.

#### Scenario: Compose service is reachable by peer agent containers on the external network

- **GIVEN** an agent container is attached to the configured external Docker network
- **WHEN** it resolves the live server endpoint from the managed bootstrap configuration
- **THEN** it can reach the service at `http://scryrs:8081`
- **AND** no host-local absolute path is required for container-to-container discovery

#### Scenario: Compose attaches to the configured external network instead of creating `scryrs-net`

- **WHEN** the consumer-facing live bootstrap compose definition is rendered
- **THEN** it joins a configured external Docker network by name
- **AND** it does not require a dedicated consumer-side `scryrs-net` network to be created and joined by agent containers

#### Scenario: Server data survives container recreation

- **GIVEN** the documented compose definition uses persistent storage for `/data/scryrs/`
- **WHEN** the scryrs server container is recreated
- **THEN** the server SQLite data remains available from the configured volume
- **AND** the live server does not revert to ephemeral in-container state

### Requirement: Repository docs describe the minimal multi-agent live setup

User-facing docs SHALL describe the workspace-local live bootstrap workflow: run live-mode `scryrs init` in the consumer project, start the managed compose stack with `scryrs up`, and ensure the scryrs server joins the existing external agent network as `scryrs`. The docs SHALL explicitly distinguish consumer bootstrap artifacts from the scryrs repository's own packaging/dev artifacts.

#### Scenario: Docs show the live init plus workspace bootstrap workflow together

- **WHEN** a reader follows the documented live setup
- **THEN** they see how live-mode `scryrs init` scaffolds `.scryrs/.env` and `.scryrs/compose.yml`
- **AND** they see how `scryrs up` starts that managed compose stack
- **AND** they see how each agent workspace resolves the shared live server as `http://scryrs:8081`

#### Scenario: Docs distinguish consumer scaffold from repository packaging artifacts

- **WHEN** a reader compares the consumer live setup guidance against the repository root Docker artifacts
- **THEN** the docs explain that the workspace-local scaffold is the supported consumer bootstrap path
- **AND** the repository root `Dockerfile` and `docker-compose.yml` are described separately as packaging or developer-maintainer artifacts
