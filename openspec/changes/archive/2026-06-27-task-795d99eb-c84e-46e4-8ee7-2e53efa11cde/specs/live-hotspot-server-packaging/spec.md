## ADDED Requirements

### Requirement: Docker packaging runs `scryrs server` as a containerized live-ingest service

The repository SHALL provide Docker packaging for the existing `scryrs server` command: a build artifact definition that produces a runnable image and defaults to `scryrs server --bind 0.0.0.0 --port 8081 --store /data/scryrs/server.db`.

#### Scenario: Container runtime uses the live-server defaults

- **WHEN** a developer starts the documented server container
- **THEN** the process inside the container runs `scryrs server`
- **AND** it binds to `0.0.0.0:8081`
- **AND** it stores server-owned SQLite data at `/data/scryrs/server.db`

### Requirement: Compose example supports persistent multi-agent networking

The repository SHALL provide a compose example for the live server with persistent SQLite storage, a stable service name `scryrs-server`, and an attachable Docker network so other agent containers can join the same network and reach the service by name.

#### Scenario: Compose service is reachable by peer agent containers

- **GIVEN** an agent container is attached to the documented Docker network
- **WHEN** it resolves the live server endpoint
- **THEN** it can reach the service at `http://scryrs-server:8081`
- **AND** no host-local absolute path is required for container-to-container discovery

#### Scenario: Server data survives container recreation

- **GIVEN** the documented compose example uses persistent storage for `/data/scryrs/`
- **WHEN** the `scryrs-server` container is recreated
- **THEN** the server SQLite data remains available from the configured volume
- **AND** the live server does not revert to ephemeral in-container state

### Requirement: Repository docs describe the minimal multi-agent live setup

User-facing docs SHALL describe a minimal end-to-end workflow for the packaged live server: start the containerized service, run live-mode init in agent workspaces with the server URL and identities, and verify live ingest/query behavior.

#### Scenario: Docs show the live init plus Docker workflow together

- **WHEN** a reader follows the documented live setup
- **THEN** they see how to start the server container
- **AND** they see how each agent workspace runs live-mode init against that server
- **AND** they see how to verify that the shared live server is receiving events

### Requirement: Scope is limited to local packaging and examples

This packaging change SHALL cover repository Docker artifacts and documentation only. It SHALL NOT add auth, TLS, hosted deployment, Kubernetes manifests, or automatic image publication.

#### Scenario: No hosted-deployment features are introduced

- **WHEN** this change is implemented
- **THEN** the repository gains Docker packaging and examples for local or team-managed deployment
- **AND** it does not introduce auth, TLS termination, Kubernetes, or hosted release automation as part of this task