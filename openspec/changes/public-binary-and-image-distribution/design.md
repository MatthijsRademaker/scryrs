## Context

scryrs currently has three disconnected distribution gaps that share one enabler (a private repo):

1. **CLI install** is source-only. `scripts/install` runs `cargo build` and requires Rust 1.85+; there is no prebuilt-binary path.
2. **Release workflow** (`.github/workflows/release.yml`) builds only `linux-x86_64` and uploads to *GitHub Actions artifacts* — ephemeral, auth-gated, no stable URL. Not usable by a `curl | sh` installer.
3. **Server image** is built locally (`Dockerfile` + `docker-compose.yml`) but never published. `live_bootstrap.rs:8` scaffolds consumer compose with `image: scryrs-server:latest` — a local-only name, so `scryrs up` cannot pull anything for a non-source consumer.

Making the repository public unlocks anonymous Release-asset downloads, anonymous `ghcr.io` pulls, and `raw.githubusercontent.com`-hosted scripts. This change wires all three into one pipeline.

Constraints: maintainer dev/test runs through Docker-backed scripts (no host SDK); workspace lints forbid `unwrap/expect/print` in Rust (shell scripts and CI YAML are unaffected); the existing source installer and dev compose/Dockerfile must remain for contributors.

## Goals / Non-Goals

**Goals:**
- Anonymous one-shot `curl -fsSL .../install.sh | sh` install for macOS arm64 and Linux x86_64.
- Tag-triggered GitHub Release with checksummed, attested, per-target-triple binary assets.
- Tag-triggered publication of `ghcr.io/matthijsrademaker/scryrs-server` (`linux/amd64`, version + `latest`).
- `scryrs up` works for a consumer who only installed the CLI — no source build required.

**Non-Goals:**
- Auth, TLS, hosted deployment, or Kubernetes manifests for the server.
- Linux arm64 / macOS x86_64 binaries or multi-arch images (deferred; `linux/amd64` and the two chosen triples only).
- Package-manager distribution (Homebrew, apt, crates.io publish).
- Pinning the consumer compose image to a specific version — `latest` is used by decision below.

## Decisions

### D1 — Publish GitHub Releases, not Actions artifacts
Rewrite `release.yml` to attach assets to a GitHub Release (e.g. `softprops/action-gh-release` or `gh release upload`). Releases give permanent, anonymous, stable download URLs the installer can target. *Alternative considered*: keep artifacts + a download proxy — rejected, artifacts expire and need auth.

### D2 — Build matrix on native runners
macOS arm64 builds on `macos-14` (Apple Silicon); Linux x86_64 builds on `ubuntu-latest`. Native runners avoid cross-compilation toolchain setup. Each job names its output `scryrs-<target-triple>` and emits a `.sha256` next to it. *Alternative*: cross-compile both from Linux — rejected, macOS codesigning/linking is simpler on a native runner.

### D3 — Installer is a standalone repo-root `install.sh`
A new self-contained script (not an extension of `scripts/install`). It: `uname`-detects OS/arch → maps to target triple → resolves the latest (or tag-pinned via env) Release asset URL → downloads binary + `.sha256` → verifies (`shasum -a 256` / `sha256sum`) → installs to `$HOME/.local/bin` (override via `--bin-dir`/`SCRYRS_INSTALL_DIR`) → `scryrs --version` → PATH guidance. `scripts/install` stays for contributors building from source. *Alternative*: dual-mode single script — rejected, mixing source-build and download logic complicates both paths.

### D4 — ghcr publish as a separate job on the same trigger
A `publish-image` job in `release.yml` logs into ghcr with `GITHUB_TOKEN` (`packages: write`), builds from the existing `Dockerfile` for `linux/amd64`, tags `:<version>` and `:latest`, pushes. Reuses the existing Dockerfile unchanged. *Alternative*: separate workflow file — folded into `release.yml` to keep one tag-driven pipeline.

### D5 — Consumer compose references `:latest` (decision A)
`live_bootstrap.rs` emits `image: ghcr.io/matthijsrademaker/scryrs-server:latest`. Chosen for simplicity; consumers always get the current server. Accepted trade-off: unpinned drift (see Risks). The image reference is a single constant in `live_bootstrap.rs`; golden/snapshot tests for the generated compose must be updated.

### D6 — `linux/amd64` only for the image (decision B)
Single-arch build avoids buildx/QEMU. The ingest server is expected on amd64 hosts. arm64 image support is a clean future addition (add `platforms:` to a buildx step) if needed.

### D7 — Make public after workflows land
Sequence: merge workflow + installer + compose changes → cut the first `v*` tag to populate a Release and ghcr package → flip repo visibility to public → set ghcr package visibility to public → update README. This ensures the first public impression already has working downloads.

## Risks / Trade-offs

- **`:latest` drift** → A consumer scaffolded months apart pulls a newer server than they tested against. Mitigation: documented; `docker compose pull` is explicit; future change can switch to CLI-version pinning if drift bites.
- **macOS Gatekeeper quarantine** on a downloaded unsigned binary → first run may warn/block. Mitigation: document the unquarantine step; codesigning is out of scope. Installs to `~/.local/bin` from `curl|sh` (not browser-downloaded) typically avoid the quarantine flag.
- **ghcr package starts private by default** → anonymous `docker pull` fails until visibility is flipped. Mitigation: explicit task in tasks.md (D7) to set package visibility public.
- **Release-asset naming drift** between workflow and installer → installer 404s. Mitigation: target-triple naming is the single contract; a smoke step in CI (or `scripts/verify-install`) exercises the installer against a published asset.
- **`curl | sh` supply-chain trust** → users pipe a script to a shell. Mitigation: repo is public and auditable; installer verifies binary checksums; document the inspect-before-run alternative (`curl ... -o install.sh`).

## Migration Plan

1. Land `release.yml` rewrite, `install.sh`, `live_bootstrap.rs` ghcr ref + updated golden tests, README/docs updates.
2. Push a `v*` tag; confirm the Release has both binaries + `.sha256` and ghcr has the image.
3. Flip repo visibility to public; set ghcr package visibility to public.
4. Verify anonymously: `curl|sh` install on macOS arm64 + Linux x86_64, and `docker pull` the image with no login.
5. Rollback: re-privatize repo / delete Release / unset package visibility; consumer compose `:latest` ref is inert until an image exists, and `scripts/install` source path is unaffected.

## Open Questions

- Should `install.sh` default to the latest Release or require a pinned version via env (`SCRYRS_VERSION`)? Lean: default latest, allow `SCRYRS_VERSION` override.
- Where is `install.sh` canonically served — repo root raw URL, or a short vanity redirect? Lean: repo-root raw URL initially; vanity domain is a later nicety.
