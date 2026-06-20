#!/usr/bin/env bash
# Shared helpers for Docker-backed verification scripts.
# Used by both host developers (local Docker) and swarm worker agents (DinD).
set -euo pipefail

# --- Docker socket selection ---
if [[ "${SWARM_DIND_ENABLED:-}" == "true" ]] && [[ -S /var/run/dind/docker.sock ]]; then
	DOCKER_SOCK="/var/run/dind/docker.sock"
	DOCKER_HOST="unix:///var/run/dind/docker.sock"
else
	DOCKER_SOCK="/var/run/docker.sock"
	DOCKER_HOST="unix:///var/run/docker.sock"
fi
export DOCKER_HOST

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PROJECT_NAME="$(basename "$ROOT")"

# Rust toolchain image. Override with RUST_IMAGE env var.
RUST_IMAGE="${RUST_IMAGE:-rust:1.85.0}"
# Security tools need a newer Rust (advisory DBs use CVSS 4.0). Override with SECURITY_RUST_IMAGE env var.
SECURITY_RUST_IMAGE="${SECURITY_RUST_IMAGE:-rust:1.88.0}"

# --- Volume names (project-scoped, DinD-isolated) ---
CACHE_VOLUME_REGISTRY="${PROJECT_NAME}-cargo-registry"
CACHE_VOLUME_GIT="${PROJECT_NAME}-cargo-git"
CACHE_VOLUME_INSTALLED="${PROJECT_NAME}-cargo-installed"

# --- Helpers ---

docker_cmd() {
	docker -H "$DOCKER_HOST" "$@"
}

# Ensure a named volume exists (idempotent).
ensure_volume() {
	local vol="$1"
	docker_cmd volume inspect "$vol" >/dev/null 2>&1 || docker_cmd volume create "$vol"
}

_pull_image_if_missing() {
	local image="$1"
	if ! docker_cmd image inspect "$image" >/dev/null 2>&1; then
		echo "[docker-verification] pulling $image ..." >&2
		docker_cmd pull "$image"
	fi
}

# Run a command inside the Rust verification container.
# Mounts the repo at /workspace with the caller's UID/GID for correct file ownership.
# Uses $RUST_IMAGE (default rust:1.85.0).
# To use a different image: run_rust --image rust:1.88.0 -- command args...
run_rust() {
	local image="$RUST_IMAGE"
	if [[ "${1:-}" == "--image" ]]; then
		image="$2"
		shift 2
	fi
	if [[ "${1:-}" == "--" ]]; then
		shift
	fi

	local uid
	uid="$(id -u)"
	local gid
	gid="$(id -g)"

	ensure_volume "$CACHE_VOLUME_REGISTRY"
	ensure_volume "$CACHE_VOLUME_GIT"
	ensure_volume "$CACHE_VOLUME_INSTALLED"
	_pull_image_if_missing "$image"

	# Ensure volume roots are writable by the non-root container user.
	# Volumes are initialized empty + root-owned; cargo needs to create
	# subdirectories (cache/, git/db/, etc.) under them.
	# This is a no-op after first run.
	docker_cmd run --rm -u root \
		-v "${CACHE_VOLUME_REGISTRY}:/usr/local/cargo/registry" \
		-v "${CACHE_VOLUME_GIT}:/usr/local/cargo/git" \
		"$image" \
		chown "${uid}:${gid}" /usr/local/cargo/registry /usr/local/cargo/git >/dev/null 2>&1 || true

	docker_cmd run --rm \
		-u "${uid}:${gid}" \
		-v "$ROOT:/workspace" \
		-v "${CACHE_VOLUME_REGISTRY}:/usr/local/cargo/registry" \
		-v "${CACHE_VOLUME_GIT}:/usr/local/cargo/git" \
		-v "${CACHE_VOLUME_INSTALLED}:/usr/local/cargo-installed" \
		-w /workspace \
		-e CARGO_HOME=/usr/local/cargo \
		-e CARGO_TERM_COLOR=always \
		-e PATH="/usr/local/cargo-installed/bin:/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
		"$image" \
		"$@"
}

# Ensure a cargo-installed binary is available (cached in a volume).
# Usage: ensure_cargo_tool cargo-deny [version] [image]
ensure_cargo_tool() {
	local tool="$1"
	local version="${2:-}"
	local image="${3:-$SECURITY_RUST_IMAGE}"
	local bin_path="/usr/local/cargo-installed/bin/${tool}"

	# Check if already installed in the cached volume
	if docker_cmd run --rm \
		-v "${CACHE_VOLUME_INSTALLED}:/usr/local/cargo-installed" \
		"$image" \
		test -x "$bin_path" 2>/dev/null; then
		return 0
	fi

	local install_args=("$tool")
	if [[ -n "$version" ]]; then
		install_args+=(--version "$version")
	fi

	echo "[docker-verification] installing ${tool}${version:+ ${version}} (cached) ..." >&2
	_pull_image_if_missing "$image"
	docker_cmd run --rm \
		-v "${CACHE_VOLUME_INSTALLED}:/usr/local/cargo-installed" \
		-e CARGO_HOME=/usr/local/cargo \
		-e PATH="/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
		"$image" \
		cargo install --locked --root /usr/local/cargo-installed "${install_args[@]}"
}

# Print a header for a verification step.
step() {
	echo "==> $*" >&2
}
