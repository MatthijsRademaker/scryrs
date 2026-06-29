#!/usr/bin/env bash
set -euo pipefail

SCRYRS_BIN="${1:-${SCRYRS_BIN:-scryrs}}"
FIXTURE_ROOT="$(mktemp -d)"
REPO_ROOT="$FIXTURE_ROOT/repo"
EVENTS_PATH="$FIXTURE_ROOT/events.jsonl"
ACCEPT_DECIDED_AT="2026-06-29T12:00:00Z"

cleanup() {
	rm -rf "$FIXTURE_ROOT"
}
trap cleanup EXIT

mkdir -p "$REPO_ROOT"

cat >"$EVENTS_PATH" <<'JSONL'
{"schema_version":"0.1.0","timestamp":"2026-06-29T12:00:00Z","session_id":"core-artifact-loop-1","event_type":"FileOpened","tool_name":"read","payload":{"type":"FileOpened","path":"src/auth.ts"},"outcome":{"result":"Success"}}
{"schema_version":"0.1.0","timestamp":"2026-06-29T12:00:01Z","session_id":"core-artifact-loop-1","event_type":"SearchRun","tool_name":"grep","payload":{"type":"SearchRun","query":"auth"},"outcome":{"result":"Success"}}
JSONL

cd "$REPO_ROOT"

"$SCRYRS_BIN" record --file "$EVENTS_PATH" >/tmp/scryrs-core-artifact-loop-record.json
[[ -f .scryrs/scryrs.db ]]
grep -F '"command":"record"' /tmp/scryrs-core-artifact-loop-record.json >/dev/null

"$SCRYRS_BIN" hotspots . >/tmp/scryrs-core-artifact-loop-hotspots.json
[[ -f .scryrs/hotspots.json ]]
grep -F '"command":"hotspots"' /tmp/scryrs-core-artifact-loop-hotspots.json >/dev/null

"$SCRYRS_BIN" graph . >/tmp/scryrs-core-artifact-loop-graph.json
[[ -f .scryrs/graph.json ]]
grep -F '"schemaVersion":"1.0.0"' /tmp/scryrs-core-artifact-loop-graph.json >/dev/null

"$SCRYRS_BIN" route . >/tmp/scryrs-core-artifact-loop-routes.json
[[ -f .scryrs/routes.json ]]
grep -F '"schemaVersion":"1.0.0"' /tmp/scryrs-core-artifact-loop-routes.json >/dev/null

proposal_count="$($SCRYRS_BIN propose . | tr -d '\r\n')"
[[ "$proposal_count" =~ ^[1-9][0-9]*$ ]]
[[ -d .scryrs/proposals ]]
proposal_path="$(find .scryrs/proposals -maxdepth 1 -type f -name '*.json' | sort | head -n 1)"
[[ -n "$proposal_path" ]]
proposal_id="$(basename "$proposal_path" .json)"

"$SCRYRS_BIN" proposals accept . "$proposal_id" --reviewer verify-production-suite --rationale "accept deterministic fixture" --decided-at "$ACCEPT_DECIDED_AT"
[[ -d .scryrs/accepted ]]
[[ -f ".scryrs/accepted/$proposal_id.json" ]]

accepted_rows="$($SCRYRS_BIN proposals list . --state accepted)"
grep -F "\"proposalId\":\"$proposal_id\"" <<<"$accepted_rows" >/dev/null
grep -F '"state":"accepted"' <<<"$accepted_rows" >/dev/null

echo "[verify-core-artifact-loop] deterministic artifact loop passed"
