## 1. Release workflow — multi-platform CLI assets

- [x] 1.1 Rewrite `.github/workflows/release.yml` `build-release` into a matrix over `macos-14` (`aarch64-apple-darwin`) and `ubuntu-latest` (`x86_64-unknown-linux-gnu`), building `cargo build -p scryrs-cli --release --locked --all-features`
- [x] 1.2 Name each built binary `scryrs-<target-triple>` and generate a sibling `scryrs-<target-triple>.sha256` (use `shasum -a 256` on macOS, `sha256sum` on Linux)
- [x] 1.3 Attach both binaries and their `.sha256` files as assets on the GitHub Release for the pushed tag (replace the `upload-artifact` step with Release publishing)
- [x] 1.4 Preserve `attest-build-provenance` for the published binary subjects
- [x] 1.5 Confirm workflow permissions: `contents: write`, `id-token: write`, `attestations: write`

## 2. Release workflow — ghcr image publication

- [x] 2.1 Add a `publish-image` job to `release.yml` triggered on the same `v*` tag with `packages: write` permission
- [x] 2.2 Log into `ghcr.io` using `GITHUB_TOKEN`, build from the repo `Dockerfile` for `linux/amd64`
- [x] 2.3 Tag and push `ghcr.io/matthijsrademaker/scryrs-server:<version>` and `:latest`

## 3. One-shot installer

- [x] 3.1 Create repo-root `install.sh`: `uname`-detect OS/arch, map to `aarch64-apple-darwin` / `x86_64-unknown-linux-gnu`, exit non-zero on unsupported platforms with no mutation
- [x] 3.2 Resolve the matching Release asset URL (default latest; honor optional `SCRYRS_VERSION` override) and download binary + `.sha256`
- [x] 3.3 Verify the binary against its `.sha256`; abort non-zero on mismatch without installing
- [x] 3.4 Install to `$HOME/.local/bin` (override via `--bin-dir`/`SCRYRS_INSTALL_DIR`), `chmod +x`, run `scryrs --version`, print PATH guidance when off-PATH
- [x] 3.5 Update `scripts/verify-install` (or add a check) to exercise `install.sh` against a published asset

## 4. Consumer compose references published image

- [x] 4.1 Change the image constant in `crates/scryrs-cli/src/live_bootstrap.rs` from `scryrs-server:latest` to `ghcr.io/matthijsrademaker/scryrs-server:latest`; ensure no `build:` context is emitted in the consumer scaffold
- [x] 4.2 Update golden/snapshot tests covering the generated `.scryrs/compose.yml`
- [x] 4.3 Run `scripts/precommit-run` (fmt, check, clippy, test) until green

## 5. Documentation

- [x] 5.1 Update `README.md` install section to lead with the `curl -fsSL .../install.sh | sh` one-shot path; keep source install as the contributor path
- [x] 5.2 Update `.devagent/docs` live-server setup to document pulling/`scryrs up` against the published `ghcr.io` image and note `:latest` drift
- [x] 5.3 Update `Dockerfile` / `docker-compose.yml` header comments to point at the published image as the canonical consumer artifact (dev/maintainer build retained)
- [x] 5.4 Document the macOS Gatekeeper unquarantine note for the downloaded binary

## 6. Go public & verify (post-merge operational)

- [ ] 6.1 Push a `v*` tag; confirm the Release has both binaries + `.sha256` and ghcr has `:<version>` and `:latest`
- [ ] 6.2 Flip repository visibility to public
- [ ] 6.3 Set the `scryrs-server` ghcr package visibility to public
- [ ] 6.4 Verify anonymously: `curl|sh` install on macOS arm64 and Linux x86_64, and `docker pull ghcr.io/matthijsrademaker/scryrs-server:latest` with no `docker login`
