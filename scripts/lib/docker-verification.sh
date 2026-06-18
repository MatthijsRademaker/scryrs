#!/usr/bin/env bash
# Shared Docker-backed verification harness.
# Source this file from scripts that execute verification inside Docker.

set -euo pipefail

VERIFY_HARNESS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERIFY_DEFAULT_WORKSPACE="$(cd "$VERIFY_HARNESS_DIR/../.." && pwd)"
VERIFY_CACHE_INIT_IMAGE="${VERIFY_CACHE_INIT_IMAGE:-alpine:3.20}"
VERIFY_VOLUME_PREFIX="${VERIFY_VOLUME_PREFIX:-dev-swarm-verify}"
VERIFY_POSTGRES_IMAGE="${POSTGRES_TEST_IMAGE:-${VERIFY_POSTGRES_IMAGE:-postgres:16-alpine}}"
VERIFY_POSTGRES_DB="${VERIFY_POSTGRES_DB:-testdb}"
VERIFY_POSTGRES_USER="${VERIFY_POSTGRES_USER:-test}"
VERIFY_POSTGRES_PASSWORD="${VERIFY_POSTGRES_PASSWORD:-test}"
VERIFY_POSTGRES_READINESS_TIMEOUT_SECONDS="${VERIFY_POSTGRES_READINESS_TIMEOUT_SECONDS:-30}"
VERIFY_POSTGRES_SIDECAR_PRESERVE="${VERIFY_POSTGRES_SIDECAR_PRESERVE:-0}"

VERIFY_DOCKER_ARGS=()
VERIFY_ENV_ARGS=()
VERIFY_CACHE_SPECS=()
VERIFY_WORKDIR="/workspace"
VERIFY_USE_DOCKER_SOCKET=0
VERIFY_MOUNT_WORKSPACE=1
VERIFY_RUN_AS_ROOT=0
VERIFY_SKIP_DOCKER_SOCKET_PREFLIGHT=0
VERIFY_POSTGRES_SIDECAR_ACTIVE=0
VERIFY_POSTGRES_CONTAINER_NAME=""
VERIFY_POSTGRES_NETWORK_NAME=""
VERIFY_POSTGRES_RUN_ID=""

verification_sanitize_name() {
	local value="$1"
	value="$(printf '%s' "$value" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9_.-]+/-/g; s/^-+//; s/-+$//; s/-+/-/g')"
	if [ -z "$value" ]; then
		value="default"
	fi
	printf '%s' "$value"
}

verification_workspace_root() {
	local candidate=""
	if [ -n "${AGENT_PROJECT_WORK_DIR:-}" ]; then
		candidate="$AGENT_PROJECT_WORK_DIR"
	elif [ -n "${PROJECT_WORK_DIR:-}" ]; then
		candidate="$PROJECT_WORK_DIR"
	else
		candidate="$VERIFY_DEFAULT_WORKSPACE"
	fi
	(cd "$candidate" && pwd)
}

verification_workspace_uid() {
	printf '%s' "${HOST_UID:-$(id -u)}"
}

verification_workspace_gid() {
	printf '%s' "${HOST_GID:-$(id -g)}"
}

verification_project_key() {
	local from_env="${SWARM_PROJECT_ID:-${PROJECT_ID:-${COMPOSE_PROJECT_NAME:-}}}"
	if [ -n "$from_env" ]; then
		printf 'swarm-%s' "$(verification_sanitize_name "$from_env")"
		return
	fi

	local root env_file project_id=""
	root="$(verification_workspace_root)"
	env_file="$root/.devagent/.env"
	if [ -f "$env_file" ]; then
		project_id="$(awk -F= '/^[[:space:]]*PROJECT_ID[[:space:]]*=/{gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); gsub(/^"|"$/, "", $2); print $2; exit}' "$env_file")"
		if [ -n "$project_id" ]; then
			printf 'swarm-%s' "$(verification_sanitize_name "$project_id")"
			return
		fi
	fi

	local base hash
	base="$(basename "$root")"
	hash="$(printf '%s' "$root" | cksum | awk '{print $1}')"
	printf 'local-%s-%s' "$(verification_sanitize_name "$base")" "$hash"
}

verification_cache_volume_name() {
	local cache_name="$1"
	printf '%s-%s-%s' "$VERIFY_VOLUME_PREFIX" "$(verification_project_key)" "$(verification_sanitize_name "$cache_name")"
}

verification_run_id() {
	if [ -z "$VERIFY_POSTGRES_RUN_ID" ]; then
		VERIFY_POSTGRES_RUN_ID="$(verification_short_name_component "$(verification_sanitize_name "${VERIFY_RUN_ID:-$$-${RANDOM:-0}}")" 11)"
	fi
	printf '%s' "$VERIFY_POSTGRES_RUN_ID"
}

verification_short_name_component() {
	local value="$1" max_len="$2" hash head_len
	if [ "${#value}" -le "$max_len" ]; then
		printf '%s' "$value"
		return
	fi
	hash="$(printf '%s' "$value" | cksum | awk '{print $1}')"
	head_len=$((max_len - ${#hash} - 1))
	if [ "$head_len" -lt 1 ]; then
		head_len=1
	fi
	printf '%s-%s' "${value:0:$head_len}" "$hash"
}

verification_postgres_sidecar_url() {
	if [ -z "$VERIFY_POSTGRES_CONTAINER_NAME" ]; then
		docker_infra_failure_emit "postgres-sidecar-url-missing" "Postgres sidecar URL requested before sidecar start."
		return 1
	fi
	printf 'postgres://%s:%s@%s:5432/%s?sslmode=disable' "$VERIFY_POSTGRES_USER" "$VERIFY_POSTGRES_PASSWORD" "$VERIFY_POSTGRES_CONTAINER_NAME" "$VERIFY_POSTGRES_DB"
}

verification_start_postgres_sidecar() {
	verification_require_docker

	local project_key name_project_key run_id network_name container_name docker_output exit_code=0
	project_key="$(verification_project_key)"
	name_project_key="$(verification_short_name_component "$project_key" 24)"
	run_id="$(verification_run_id)"
	network_name="$VERIFY_VOLUME_PREFIX-$name_project_key-net-$run_id"
	container_name="$VERIFY_VOLUME_PREFIX-$name_project_key-postgres-$run_id"
	VERIFY_POSTGRES_NETWORK_NAME="$network_name"
	VERIFY_POSTGRES_CONTAINER_NAME="$container_name"

	docker_output=$(docker network create \
		--label dev-swarm.verification-sidecar=true \
		--label dev-swarm.verification-purpose=postgres-sidecar \
		--label "dev-swarm.project-key=$project_key" \
		--label "dev-swarm.verification-run=$run_id" \
		"$network_name" 2>&1) || exit_code=$?
	if [ "$exit_code" -ne 0 ]; then
		docker_infra_failure_emit "postgres-sidecar-network-create-failed" "$docker_output"
		return "$exit_code"
	fi
	VERIFY_POSTGRES_SIDECAR_ACTIVE=1

	exit_code=0
	docker_output=$(docker run -d \
		--name "$container_name" \
		--network "$network_name" \
		--network-alias "$container_name" \
		--label dev-swarm.verification-sidecar=true \
		--label dev-swarm.verification-purpose=postgres-sidecar \
		--label "dev-swarm.project-key=$project_key" \
		--label "dev-swarm.verification-run=$run_id" \
		-e "POSTGRES_DB=$VERIFY_POSTGRES_DB" \
		-e "POSTGRES_USER=$VERIFY_POSTGRES_USER" \
		-e "POSTGRES_PASSWORD=$VERIFY_POSTGRES_PASSWORD" \
		"$VERIFY_POSTGRES_IMAGE" 2>&1) || exit_code=$?
	if [ "$exit_code" -ne 0 ]; then
		if _docker_is_image_failure "$docker_output"; then
			docker_infra_failure_emit "postgres-sidecar-image-unavailable" "$docker_output"
		else
			docker_infra_failure_emit "postgres-sidecar-start-failed" "$docker_output"
		fi
		return "$exit_code"
	fi

	verification_wait_for_postgres_sidecar
}

# ── Reusable Postgres sidecar (no run-id suffix, survives across invocations) ──

verification_reuse_or_start_postgres_sidecar() {
	verification_require_docker

	local project_key name_project_key network_name container_name docker_output exit_code=0
	project_key="$(verification_project_key)"
	name_project_key="$(verification_short_name_component "$project_key" 24)"
	network_name="$VERIFY_VOLUME_PREFIX-$name_project_key-postgres-net"
	container_name="$VERIFY_VOLUME_PREFIX-$name_project_key-postgres"
	VERIFY_POSTGRES_NETWORK_NAME="$network_name"
	VERIFY_POSTGRES_CONTAINER_NAME="$container_name"
	VERIFY_POSTGRES_SIDECAR_ACTIVE=1

	# Ensure network exists
	if ! docker network inspect "$network_name" >/dev/null 2>&1; then
		docker_output=$(docker network create \
			--label dev-swarm.verification-sidecar=true \
			--label dev-swarm.verification-purpose=postgres-sidecar \
			--label "dev-swarm.project-key=$project_key" \
			"$network_name" 2>&1) || exit_code=$?
		if [ "$exit_code" -ne 0 ]; then
			docker_infra_failure_emit "postgres-sidecar-network-create-failed" "$docker_output"
			return "$exit_code"
		fi
	fi

	# Check container state
	local container_status
	container_status=$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || echo "absent")

	case "$container_status" in
	running)
		# Already running — verify it's healthy
		verification_wait_for_postgres_sidecar
		return $?
		;;
	exited|created)
		# Exists but stopped — start it
		docker start "$container_name" >/dev/null 2>&1 || exit_code=$?
		if [ "$exit_code" -ne 0 ]; then
			docker_infra_failure_emit "postgres-sidecar-start-failed" "Failed to start existing postgres container $container_name"
			return "$exit_code"
		fi
		verification_wait_for_postgres_sidecar
		return $?
		;;
	esac

	# Container absent — create fresh
	exit_code=0
	docker_output=$(docker run -d \
		--name "$container_name" \
		--network "$network_name" \
		--network-alias "$container_name" \
		--label dev-swarm.verification-sidecar=true \
		--label dev-swarm.verification-purpose=postgres-sidecar \
		--label "dev-swarm.project-key=$project_key" \
		-e "POSTGRES_DB=$VERIFY_POSTGRES_DB" \
		-e "POSTGRES_USER=$VERIFY_POSTGRES_USER" \
		-e "POSTGRES_PASSWORD=$VERIFY_POSTGRES_PASSWORD" \
		"$VERIFY_POSTGRES_IMAGE" 2>&1) || exit_code=$?
	if [ "$exit_code" -ne 0 ]; then
		if _docker_is_image_failure "$docker_output"; then
			docker_infra_failure_emit "postgres-sidecar-image-unavailable" "$docker_output"
		else
			docker_infra_failure_emit "postgres-sidecar-start-failed" "$docker_output"
		fi
		return "$exit_code"
	fi

	verification_wait_for_postgres_sidecar
}

verification_wait_for_postgres_sidecar() {
	local deadline now
	deadline=$(($(date +%s) + VERIFY_POSTGRES_READINESS_TIMEOUT_SECONDS))
	while true; do
		if docker exec -e "PGPASSWORD=$VERIFY_POSTGRES_PASSWORD" "$VERIFY_POSTGRES_CONTAINER_NAME" pg_isready -U "$VERIFY_POSTGRES_USER" -d "$VERIFY_POSTGRES_DB" >/dev/null 2>&1; then
			return 0
		fi
		now=$(date +%s)
		if [ "$now" -ge "$deadline" ]; then
			docker_infra_failure_emit "postgres-sidecar-readiness-timeout" "Postgres sidecar $VERIFY_POSTGRES_CONTAINER_NAME did not become ready within ${VERIFY_POSTGRES_READINESS_TIMEOUT_SECONDS}s."
			return 1
		fi
		sleep 1
	done
}

verification_cleanup_postgres_sidecar() {
	if [ "$VERIFY_POSTGRES_SIDECAR_ACTIVE" -ne 1 ]; then
		return 0
	fi
	if [ "$VERIFY_POSTGRES_SIDECAR_PRESERVE" = "1" ]; then
		echo "[INFO] Preserving Postgres sidecar resources: container=$VERIFY_POSTGRES_CONTAINER_NAME network=$VERIFY_POSTGRES_NETWORK_NAME" >&2
		return 0
	fi
	if [ -n "$VERIFY_POSTGRES_CONTAINER_NAME" ]; then
		docker stop "$VERIFY_POSTGRES_CONTAINER_NAME" >/dev/null 2>&1 || true
		docker rm -f "$VERIFY_POSTGRES_CONTAINER_NAME" >/dev/null 2>&1 || true
	fi
	if [ -n "$VERIFY_POSTGRES_NETWORK_NAME" ]; then
		docker network rm "$VERIFY_POSTGRES_NETWORK_NAME" >/dev/null 2>&1 || true
	fi
}

verification_reset() {
	VERIFY_DOCKER_ARGS=()
	VERIFY_ENV_ARGS=()
	VERIFY_CACHE_SPECS=()
	VERIFY_WORKDIR="/workspace"
	VERIFY_USE_DOCKER_SOCKET=0
	VERIFY_MOUNT_WORKSPACE=1
	VERIFY_RUN_AS_ROOT=0
	VERIFY_SKIP_DOCKER_SOCKET_PREFLIGHT=0
}

verification_workdir() {
	VERIFY_WORKDIR="$1"
}

verification_without_workspace_mount() {
	VERIFY_MOUNT_WORKSPACE=0
}

verification_with_docker_socket() {
	VERIFY_USE_DOCKER_SOCKET=1
}

verification_run_as_root() {
	VERIFY_RUN_AS_ROOT=1
}

verification_skip_docker_socket_preflight() {
	VERIFY_SKIP_DOCKER_SOCKET_PREFLIGHT=1
}

verification_add_env() {
	VERIFY_ENV_ARGS+=("-e" "$1")
}

verification_add_arg() {
	VERIFY_DOCKER_ARGS+=("$@")
}

verification_add_cache() {
	local cache_name="$1"
	local mount_path="$2"
	local env_name="${3:-}"
	local volume
	volume="$(verification_cache_volume_name "$cache_name")"
	VERIFY_CACHE_SPECS+=("$volume:$mount_path")
	VERIFY_DOCKER_ARGS+=("-v" "$volume:$mount_path")
	if [ -n "$env_name" ]; then
		VERIFY_ENV_ARGS+=("-e" "$env_name=$mount_path")
	fi
}

verification_docker_socket_gid() {
	local socket="$1"
	if stat -c '%g' "$socket" >/dev/null 2>&1; then
		stat -c '%g' "$socket"
	else
		stat -f '%g' "$socket"
	fi
}

verification_require_docker() {
	if ! command -v docker >/dev/null 2>&1; then
		echo "[ERROR] Docker CLI is not installed or is not on PATH." >&2
		exit 1
	fi

	if [ -n "${DOCKER_HOST:-}" ]; then
		case "$DOCKER_HOST" in
		unix://*)
			VERIFY_DOCKER_HOST="$DOCKER_HOST"
			VERIFY_DOCKER_SOCK="${DOCKER_HOST#unix://}"
			;;
		*)
			echo "[ERROR] Unsupported DOCKER_HOST '$DOCKER_HOST'. This verification harness supports unix:// Docker sockets only." >&2
			exit 1
			;;
		esac
	elif [ -S /var/run/dind/docker.sock ]; then
		VERIFY_DOCKER_SOCK="/var/run/dind/docker.sock"
		VERIFY_DOCKER_HOST="unix://$VERIFY_DOCKER_SOCK"
	elif [ -S /var/run/docker.sock ]; then
		VERIFY_DOCKER_SOCK="/var/run/docker.sock"
		VERIFY_DOCKER_HOST="unix://$VERIFY_DOCKER_SOCK"
	else
		echo "[ERROR] No Docker socket found. Set DOCKER_HOST=unix:///path/to/docker.sock or expose /var/run/docker.sock." >&2
		exit 1
	fi

	if [ ! -S "$VERIFY_DOCKER_SOCK" ]; then
		docker_infra_failure_emit "docker-socket-missing" "Docker socket does not exist or is not a unix socket: $VERIFY_DOCKER_SOCK"
		exit 1
	fi

	if ! docker info >/dev/null 2>&1; then
		docker_infra_failure_emit "docker-daemon-unavailable" "Docker daemon is unavailable or the socket is inaccessible: $VERIFY_DOCKER_HOST"
		echo "[HINT] Ensure Docker is running and DOCKER_HOST points at a reachable unix socket." >&2
		exit 1
	fi
}

verification_init_cache_volumes() {
	local uid gid spec volume mount_path project_key docker_errfile docker_output exit_code=0
	uid="$(verification_workspace_uid)"
	gid="$(verification_workspace_gid)"
	project_key="$(verification_project_key)"
	docker_errfile=$(mktemp)

	if [ "${#VERIFY_CACHE_SPECS[@]}" -eq 0 ]; then
		rm -f "$docker_errfile"
		return 0
	fi

	for spec in "${VERIFY_CACHE_SPECS[@]}"; do
		volume="${spec%%:*}"
		mount_path="${spec#*:}"
		docker volume create \
			--label dev-swarm.verification-cache=true \
			--label "dev-swarm.project-key=$project_key" \
			"$volume" >/dev/null 2>>"$docker_errfile" || exit_code=$?
		if [ "$exit_code" -ne 0 ]; then
			docker_output=$(cat "$docker_errfile")
			rm -f "$docker_errfile"
			if _docker_is_daemon_failure "$docker_output"; then
				docker_infra_failure_emit "docker-daemon-unavailable" "$docker_output"
			else
				printf '%s\n' "$docker_output" >&2
			fi
			exit "$exit_code"
		fi
		docker run --rm \
			-v "$volume:$mount_path" \
			"$VERIFY_CACHE_INIT_IMAGE" \
			sh -c 'uid="$1"; gid="$2"; shift 2; for dir in "$@"; do mkdir -p "$dir"; chown -R "$uid:$gid" "$dir"; done' \
			sh "$uid" "$gid" "$mount_path" >/dev/null 2>>"$docker_errfile" || exit_code=$?
		if [ "$exit_code" -ne 0 ]; then
			docker_output=$(cat "$docker_errfile")
			rm -f "$docker_errfile"
			if _docker_is_daemon_failure "$docker_output"; then
				docker_infra_failure_emit "docker-daemon-unavailable" "$docker_output"
			elif _docker_is_image_failure "$docker_output"; then
				docker_infra_failure_emit "verifier-image-unavailable" "$docker_output"
			else
				printf '%s\n' "$docker_output" >&2
			fi
			exit "$exit_code"
		fi
	done
	rm -f "$docker_errfile"
}

verification_run_container() {
	local image="$1"
	shift

	verification_require_docker
	verification_init_cache_volumes

	local uid gid workspace run_args socket_gid
	uid="$(verification_workspace_uid)"
	gid="$(verification_workspace_gid)"
	workspace="$(verification_workspace_root)"

	run_args=(--rm)
	if [ "$VERIFY_RUN_AS_ROOT" -eq 0 ]; then
		run_args+=(--user "$uid:$gid")
	fi
	if [ "$VERIFY_MOUNT_WORKSPACE" -eq 1 ]; then
		run_args+=("-v" "$workspace:/workspace" "-w" "$VERIFY_WORKDIR")
	else
		run_args+=("-w" "$VERIFY_WORKDIR")
	fi

	if [ "$VERIFY_USE_DOCKER_SOCKET" -eq 1 ]; then
		run_args+=("-v" "$VERIFY_DOCKER_SOCK:$VERIFY_DOCKER_SOCK" "-e" "DOCKER_HOST=$VERIFY_DOCKER_HOST")
		socket_gid="$(verification_docker_socket_gid "$VERIFY_DOCKER_SOCK" 2>/dev/null || true)"
		run_args+=("--group-add" "0")
		if [ -n "$socket_gid" ] && [ "$socket_gid" != "0" ]; then
			run_args+=("--group-add" "$socket_gid")
		fi
	fi

	# Capture outer Docker stderr to detect infrastructure failures while
	# preserving ordinary verifier-container stdout/stderr.
	local docker_errfile docker_output exit_code=0
	docker_errfile=$(mktemp)

	if [ "$VERIFY_USE_DOCKER_SOCKET" -eq 1 ] && [ "$VERIFY_SKIP_DOCKER_SOCKET_PREFLIGHT" -eq 0 ]; then
		docker run "${run_args[@]}" "${VERIFY_DOCKER_ARGS[@]}" "${VERIFY_ENV_ARGS[@]}" "$image" \
			sh -c 'sock="${DOCKER_HOST#unix://}"; if [ ! -S "$sock" ]; then echo "verifier Docker socket propagation failed: $sock" >&2; exit 86; fi; docker info >/dev/null' 2>"$docker_errfile" || exit_code=$?
		if [ "$exit_code" -ne 0 ]; then
			docker_output=$(cat "$docker_errfile")
			rm -f "$docker_errfile"
			if _docker_is_daemon_failure "$docker_output"; then
				docker_infra_failure_emit "docker-daemon-unavailable" "$docker_output"
			elif _docker_is_image_failure "$docker_output"; then
				docker_infra_failure_emit "verifier-image-unavailable" "$docker_output"
			else
				docker_infra_failure_emit "verifier-socket-propagation-failed" "$docker_output"
			fi
			return "$exit_code"
		fi
		: >"$docker_errfile"
	fi

	docker run "${run_args[@]}" "${VERIFY_DOCKER_ARGS[@]}" "${VERIFY_ENV_ARGS[@]}" "$image" "$@" 2>"$docker_errfile" || exit_code=$?

	if [ "$exit_code" -ne 0 ]; then
		docker_output=$(cat "$docker_errfile")
		rm -f "$docker_errfile"
		if _docker_is_daemon_failure "$docker_output"; then
			docker_infra_failure_emit "docker-daemon-unavailable" "$docker_output"
		elif _docker_is_image_failure "$docker_output"; then
			docker_infra_failure_emit "verifier-image-unavailable" "$docker_output"
		elif [ "$VERIFY_POSTGRES_SIDECAR_ACTIVE" -eq 1 ] && _docker_is_network_failure "$docker_output"; then
			docker_infra_failure_emit "postgres-sidecar-network-attach-failed" "$docker_output"
		else
			printf '%s\n' "$docker_output" >&2
		fi
		return "$exit_code"
	fi

	cat "$docker_errfile" >&2
	rm -f "$docker_errfile"
}

verification_cache_volume_prefix() {
	printf '%s-%s-' "$VERIFY_VOLUME_PREFIX" "$(verification_project_key)"
}

# ---------------------------------------------------------------------------
# Docker infrastructure failure signaling
# ---------------------------------------------------------------------------

# Emit a machine-readable infrastructure failure sentinel to stderr.
# Usage: docker_infra_failure_emit <cause> <human-readable message>
docker_infra_failure_emit() {
	local cause="$1"
	shift
	printf '[DOCKER_INFRA_FAILURE:%s] %s\n' "$cause" "$*" >&2
}

# Check whether Docker error text matches a known daemon/socket failure signature.
_docker_is_daemon_failure() {
	local text="$1"
	echo "$text" | grep -qiE 'cannot connect to the docker daemon|is the docker daemon running|docker daemon is not running|error response from daemon|dial unix.*docker\.sock|permission denied.*(docker|socket)|no such file.*docker\.sock'
}

# Check whether Docker error text matches a known image-unavailable failure signature.
_docker_is_image_failure() {
	local text="$1"
	echo "$text" | grep -qiE 'pull access denied|manifest unknown|repository does not exist|requested access to the resource is denied|no such image|image not found'
}

_docker_is_network_failure() {
	local text="$1"
	echo "$text" | grep -qiE 'network .*not found|network attach failed|failed to .*network|could not attach to network|no such network'
}
