/**
 * SQLite datastore helpers for scryrs cross-harness verification fixtures.
 *
 * Reads accepted events from the canonical `.scryrs/scryrs.db` SQLite
 * datastore via Python's stdlib `sqlite3` module. This avoids native Node
 * bindings inside Linux Docker fixtures mounted over a host `node_modules/`.
 */

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { fail } from "./assert.mjs";

const SQLITE_READ_SCRIPT = String.raw`
import json
import sqlite3
import sys

conn = sqlite3.connect(sys.argv[1])
try:
    rows = conn.execute("SELECT event_json FROM trace_events ORDER BY rowid").fetchall()
    print(json.dumps([row[0] for row in rows]))
finally:
    conn.close()
`;

/**
 * Read all persisted events from a scryrs SQLite datastore.
 *
 * Returns an array of parsed TraceEvent JSON objects from the event_json
 * column of the trace_events table, ordered by insertion (rowid).
 * Returns an empty array if the database file doesn't exist.
 */
export function readEventsDb(dbPath) {
	if (!existsSync(dbPath)) {
		return [];
	}

	const result = spawnSync("python3", ["-c", SQLITE_READ_SCRIPT, dbPath], {
		encoding: "utf-8",
	});

	if (result.error) {
		fail("readEventsDb", `python3 spawn failed: ${result.error.message}`);
		return [];
	}

	if (result.status !== 0) {
		const reason = (
			result.stderr ||
			result.stdout ||
			"unknown sqlite read error"
		).trim();
		fail("readEventsDb", `SQLite query failed: ${reason}`);
		return [];
	}

	let rows;
	try {
		rows = JSON.parse(result.stdout || "[]");
	} catch (err) {
		fail("readEventsDb", `invalid Python JSON output: ${err.message}`);
		return [];
	}

	const events = [];
	for (const eventJson of rows) {
		try {
			events.push(JSON.parse(eventJson));
		} catch {
			fail("readEventsDb", `invalid JSON in event_json column`);
		}
	}
	return events;
}

/**
 * Assert that a parsed TraceEvent has the canonical envelope shape.
 *
 * Checks:
 *   - schema_version: "0.1.0"
 *   - timestamp: RFC 3339 string (non-empty)
 *   - session_id: non-empty string
 *   - event_type: matches expectedType
 *   - tool_name: matches expectedToolName (if provided)
 *   - payload: non-null object with `type` tag
 *   - outcome: non-null object with `result` field
 *
 * Returns true if all checks pass, false otherwise.
 */
export function assertEventShape(event, expectedType, expectedToolName) {
	let ok = true;
	const prefix = expectedToolName ? `${expectedToolName}` : expectedType;

	const check = (condition, msg) => {
		if (!condition) {
			ok = false;
			fail(`${prefix}: envelope shape`, msg);
		}
	};

	if (!event || typeof event !== "object") {
		fail(`${prefix}: envelope shape`, `event is not an object`);
		return false;
	}

	check(
		event.schema_version === "0.1.0",
		`schema_version=${event.schema_version}`,
	);
	check(
		typeof event.timestamp === "string" && event.timestamp.length > 0,
		"missing or empty timestamp",
	);
	check(
		typeof event.session_id === "string" && event.session_id.length > 0,
		"missing or empty session_id",
	);
	check(
		event.event_type === expectedType,
		`event_type=${event.event_type} expected=${expectedType}`,
	);

	if (expectedToolName !== undefined) {
		check(
			event.tool_name === expectedToolName,
			`tool_name=${event.tool_name} expected=${expectedToolName}`,
		);
	}

	check(
		event.payload !== null && typeof event.payload === "object",
		"payload is not an object",
	);

	if (event.payload && typeof event.payload === "object") {
		check(
			typeof event.payload.type === "string",
			"payload.type missing or not a string",
		);
	}

	check(
		event.outcome !== null &&
			typeof event.outcome === "object" &&
			typeof event.outcome.result === "string",
		"outcome missing or result not a string",
	);

	return ok;
}
