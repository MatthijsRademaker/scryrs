## ADDED Requirements

### Requirement: Scaffolded compose references the published server image

The managed `.scryrs/compose.yml` produced by live-mode `scryrs init` SHALL reference the published GitHub Container Registry server image `ghcr.io/matthijsrademaker/scryrs-server:latest` rather than a local-only image name. The scaffold SHALL NOT require the consumer to build the server image from a scryrs source checkout for `scryrs up` to start the stack.

#### Scenario: Generated compose pulls the published image

- **WHEN** live-mode `scryrs init` scaffolds `.scryrs/compose.yml` in a consumer project
- **THEN** the compose service `image` is `ghcr.io/matthijsrademaker/scryrs-server:latest`
- **AND** the compose file does not declare a `build:` context requiring a local source checkout

#### Scenario: `scryrs up` starts the stack from the published image without a source build

- **GIVEN** a consumer project with a scaffolded `.scryrs/compose.yml` and a resolvable external Docker network
- **AND** no local `scryrs-server` image has been built
- **WHEN** `scryrs up` is invoked
- **THEN** Docker Compose pulls `ghcr.io/matthijsrademaker/scryrs-server:latest`
- **AND** the live server starts without requiring a scryrs source checkout
