#!/bin/sh
# One-shot installer for the scryrs CLI.
#
# Downloads a published GitHub Release binary for the host platform, verifies it
# against its published .sha256 checksum, and installs it on PATH. No source
# checkout, Rust, or Cargo required.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/matthijsrademaker/scryrs/main/install.sh | sh
#
#   # custom install dir:
#   curl -fsSL .../install.sh | sh -s -- --bin-dir /usr/local/bin
#   curl -fsSL .../install.sh | SCRYRS_INSTALL_DIR=/usr/local/bin sh
#
#   # pin a specific release tag (default: latest):
#   curl -fsSL .../install.sh | SCRYRS_VERSION=v0.1.0 sh
#
# Supported platforms: macOS arm64 (aarch64-apple-darwin), Linux x86_64
# (x86_64-unknown-linux-gnu). Other platforms exit non-zero without mutation.
set -eu

REPO="matthijsrademaker/scryrs"
TAG="scryrs-install"

log() { echo "[$TAG] $*"; }
err() { echo "[$TAG] ERROR: $*" >&2; }
die() { err "$*"; exit 1; }

# ── Argument parsing ──────────────────────────────────────────────────────────

BIN_DIR=""
while [ $# -gt 0 ]; do
    case "$1" in
        --bin-dir)
            [ -n "${2:-}" ] || { err "--bin-dir requires a path argument."; exit 2; }
            BIN_DIR="$2"
            shift 2
            ;;
        --help|-h)
            cat <<'USAGE'
Usage: install.sh [--bin-dir <PATH>]

Install the scryrs CLI by downloading a published release binary.

Options:
  --bin-dir <PATH>   Install target directory (default: $HOME/.local/bin)

Environment:
  SCRYRS_INSTALL_DIR  Install target directory (overridden by --bin-dir)
  SCRYRS_VERSION      Release tag to install (default: latest, e.g. v0.1.0)

Supported platforms: macOS arm64, Linux x86_64.
USAGE
            exit 0
            ;;
        *)
            err "unknown argument '$1'. Run with --help for usage."
            exit 2
            ;;
    esac
done

# ── Platform detection ────────────────────────────────────────────────────────

OS="$(uname -s)"
ARCH="$(uname -m)"
TARGET=""
case "$OS" in
    Darwin)
        case "$ARCH" in
            arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
        esac
        ;;
    Linux)
        case "$ARCH" in
            x86_64|amd64) TARGET="x86_64-unknown-linux-gnu" ;;
        esac
        ;;
esac

if [ -z "$TARGET" ]; then
    err "unsupported platform: $OS/$ARCH."
    err "scryrs publishes binaries for macOS arm64 and Linux x86_64 only."
    err "Build from source instead: https://github.com/$REPO#install-from-source"
    exit 1
fi

ASSET="scryrs-$TARGET"

# ── Install directory resolution ──────────────────────────────────────────────

if [ -z "$BIN_DIR" ]; then
    if [ -n "${SCRYRS_INSTALL_DIR:-}" ]; then
        BIN_DIR="$SCRYRS_INSTALL_DIR"
    else
        BIN_DIR="$HOME/.local/bin"
    fi
fi

# ── Download URL resolution ───────────────────────────────────────────────────

# GitHub serves the latest release's assets via the /releases/latest/download
# redirect, and pinned assets via /releases/download/<tag>. Both work anonymously
# on a public repo with no API token.
if [ -n "${SCRYRS_VERSION:-}" ]; then
    BASE_URL="https://github.com/$REPO/releases/download/$SCRYRS_VERSION"
    log "Installing scryrs $SCRYRS_VERSION ($TARGET) ..."
else
    BASE_URL="https://github.com/$REPO/releases/latest/download"
    log "Installing latest scryrs ($TARGET) ..."
fi

# ── Downloader selection ──────────────────────────────────────────────────────

if command -v curl >/dev/null 2>&1; then
    download() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
    download() { wget -qO "$2" "$1"; }
else
    die "neither 'curl' nor 'wget' found on PATH; cannot download."
fi

# ── Checksum tool selection ───────────────────────────────────────────────────

if command -v sha256sum >/dev/null 2>&1; then
    verify_checksum() { sha256sum -c "$1" >/dev/null 2>&1; }
elif command -v shasum >/dev/null 2>&1; then
    verify_checksum() { shasum -a 256 -c "$1" >/dev/null 2>&1; }
else
    die "neither 'sha256sum' nor 'shasum' found on PATH; cannot verify download."
fi

# ── Download, verify, install ─────────────────────────────────────────────────

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT INT TERM

log "Downloading $ASSET ..."
download "$BASE_URL/$ASSET" "$TMP_DIR/$ASSET" \
    || die "failed to download $BASE_URL/$ASSET"
download "$BASE_URL/$ASSET.sha256" "$TMP_DIR/$ASSET.sha256" \
    || die "failed to download checksum $BASE_URL/$ASSET.sha256"

log "Verifying checksum ..."
# The .sha256 file references the asset by its basename, so verify from inside
# the temp directory where the downloaded file has exactly that name.
( cd "$TMP_DIR" && verify_checksum "$ASSET.sha256" ) \
    || die "checksum verification failed for $ASSET; refusing to install."

log "Installing scryrs to $BIN_DIR ..."
mkdir -p "$BIN_DIR"
cp "$TMP_DIR/$ASSET" "$BIN_DIR/scryrs"
chmod +x "$BIN_DIR/scryrs"

# ── Verify the installed binary runs ──────────────────────────────────────────

if ! VERSION_OUTPUT="$("$BIN_DIR/scryrs" --version 2>&1)"; then
    err "installed binary failed '--version' check."
    err "Output: $VERSION_OUTPUT"
    err "On macOS, Gatekeeper may quarantine the download. Try:"
    err "  xattr -d com.apple.quarantine \"$BIN_DIR/scryrs\""
    exit 1
fi
log "$VERSION_OUTPUT"

# ── PATH guidance ─────────────────────────────────────────────────────────────

INSTALLED_PATH="$(command -v scryrs 2>/dev/null || true)"
if [ "$INSTALLED_PATH" = "$BIN_DIR/scryrs" ]; then
    log "✓ scryrs installed and available on PATH"
else
    log "✓ scryrs installed to $BIN_DIR/scryrs"
    echo ""
    echo "The install directory is NOT on your current PATH."
    echo "Add it to your shell profile to use 'scryrs' directly:"
    echo ""
    echo "  export PATH=\"$BIN_DIR:\$PATH\""
    echo ""
    echo "Or run the binary directly:"
    echo "  $BIN_DIR/scryrs --help"
fi

echo ""
log "Done. Next: run 'scryrs init --agent <NAME>' to install agent hooks, then 'scryrs doctor' to verify."
