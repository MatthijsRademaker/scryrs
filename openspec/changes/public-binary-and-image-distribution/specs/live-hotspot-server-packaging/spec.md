## MODIFIED Requirements

### Requirement: Scope is limited to local packaging and examples

This packaging change SHALL cover repository Docker artifacts, documentation, and automatic publication of the server image to GitHub Container Registry. It SHALL NOT add auth, TLS, hosted deployment, or Kubernetes manifests.

#### Scenario: No hosted-deployment features are introduced

- **WHEN** this change is implemented
- **THEN** the repository gains Docker packaging and examples for local or team-managed deployment, plus automatic image publication to `ghcr.io`
- **AND** it does not introduce auth, TLS termination, Kubernetes, or hosted runtime deployment as part of this task

#### Scenario: Published image is the canonical server artifact

- **WHEN** the server image is published on a release tag
- **THEN** `ghcr.io/matthijsrademaker/scryrs-server` is the canonical artifact consumers pull for live ingestion
- **AND** the repository root `Dockerfile`/`docker-compose.yml` remain maintainer/dev build artifacts
