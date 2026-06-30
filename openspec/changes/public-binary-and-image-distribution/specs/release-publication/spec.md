## ADDED Requirements

### Requirement: Tag push builds and publishes multi-platform CLI release assets

On push of a `v*` tag, the release workflow SHALL build the `scryrs` CLI in release mode (locked, all features) for macOS arm64 (`aarch64-apple-darwin`) and Linux x86_64 (`x86_64-unknown-linux-gnu`), and SHALL attach each built binary plus a `.sha256` checksum to the corresponding GitHub Release as downloadable assets named by their target triple.

#### Scenario: Both platform binaries are attached to the Release

- **WHEN** a `v*` tag is pushed
- **THEN** the workflow builds the CLI for `aarch64-apple-darwin` and `x86_64-unknown-linux-gnu`
- **AND** each binary and its `.sha256` are uploaded as assets on the GitHub Release for that tag

#### Scenario: Release assets are anonymously downloadable

- **GIVEN** the repository is public and a Release with attached assets exists
- **WHEN** an unauthenticated client requests a release asset by its stable Release download URL
- **THEN** the asset is downloadable without authentication

#### Scenario: Build provenance attestation is preserved

- **WHEN** the release workflow publishes the binaries
- **THEN** build-provenance attestation is generated for the published binary subjects

### Requirement: Tag push publishes the server image to GitHub Container Registry

On push of a `v*` tag, the release workflow SHALL build the server image from the repository `Dockerfile` for `linux/amd64` and push it to `ghcr.io/matthijsrademaker/scryrs-server`, tagged with both the released version and `latest`, authenticating with the workflow `GITHUB_TOKEN` under `packages: write` permission.

#### Scenario: Image is pushed with version and latest tags

- **WHEN** a `v*` tag is pushed
- **THEN** the workflow builds the server image for `linux/amd64`
- **AND** pushes it to `ghcr.io/matthijsrademaker/scryrs-server` tagged with the version and `latest`

#### Scenario: Published image is anonymously pullable

- **GIVEN** the `ghcr.io` package visibility is public
- **WHEN** an unauthenticated client runs `docker pull ghcr.io/matthijsrademaker/scryrs-server:latest`
- **THEN** the pull succeeds without `docker login`
