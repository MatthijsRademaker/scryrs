## Why

Today scryrs can only be installed from a source checkout (`scripts/install` requires Rust/Cargo), the release workflow uploads to ephemeral GitHub Actions artifacts rather than downloadable Release assets, and the server image is built locally but never published. As a result there is no anonymous, one-shot path to install the CLI, and `scryrs up` scaffolds a compose file referencing a local-only `scryrs-server:latest` image that consumers cannot pull. Making the repository public unlocks anonymous downloads, anonymous `ghcr.io` pulls, and `raw.githubusercontent.com`-hosted install scripts — turning these gaps into a coherent, public distribution story.

## What Changes

- Rewrite the tag-triggered release workflow to build the `scryrs` CLI for **macOS arm64** (`aarch64-apple-darwin`) and **Linux x86_64** (`x86_64-unknown-linux-gnu`), name each asset by target triple with a `.sha256`, and attach them to a **GitHub Release** (not Actions artifacts), preserving build-provenance attestation.
- Add automatic publication of the server image to GitHub Container Registry on the same tag trigger: `ghcr.io/matthijsrademaker/scryrs-server` for `linux/amd64`, tagged with the released version and `latest`.
- Add a self-contained one-shot installer `install.sh` (repo root) invokable via `curl -fsSL .../install.sh | sh`: detect OS/arch, download the matching Release asset, verify its checksum, place `scryrs` on PATH, and verify with `scryrs --version`. The existing `scripts/install` source installer is retained for contributors.
- **BREAKING (consumer-visible)**: the workspace live-bootstrap compose scaffold references `image: ghcr.io/matthijsrademaker/scryrs-server:latest` instead of the local-only `scryrs-server:latest`, so `scryrs up` pulls the published image with no source build.
- Lift the "no automatic image publication" exclusion from the live-hotspot server packaging scope.
- Make the repository public (one-time operational task) and update README install guidance to lead with the one-shot installer.

## Capabilities

### New Capabilities
- `prebuilt-binary-installer`: a platform-detecting, checksum-verifying one-shot `install.sh` that downloads and installs a published `scryrs` Release binary without requiring a source checkout or Rust toolchain.
- `release-publication`: the tag-triggered release pipeline that publishes multi-platform CLI binaries as GitHub Release assets (with checksums and attestation) and publishes the `linux/amd64` server image to `ghcr.io`.

### Modified Capabilities
- `live-hotspot-server-packaging`: removes the requirement-level exclusion that forbade automatic image publication; packaging now defines a published `ghcr.io` image as the canonical server artifact.
- `workspace-live-bootstrap`: the scaffolded `.scryrs/compose.yml` references the published `ghcr.io/matthijsrademaker/scryrs-server:latest` image instead of a local-only image name.

## Impact

- **CI/CD**: `.github/workflows/release.yml` rewritten (matrix build, Release publishing, ghcr publish job). New `packages: write` permission scope.
- **Code**: `crates/scryrs-cli/src/live_bootstrap.rs` (compose image reference) and its golden/snapshot tests.
- **Scripts/docs**: new `install.sh`; README install section; `.devagent/docs` live-server setup pages; `docker-compose.yml`/`Dockerfile` header comments noting the published image.
- **Repository settings**: visibility flipped to public; ghcr package visibility set to public.
- **No change** to runtime CLI behavior, server protocol, or stored data formats.
