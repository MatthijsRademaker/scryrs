/**
 * JSONL helpers for scryrs cross-harness verification fixtures.
 */

import { readFileSync, existsSync } from "node:fs";
import { fail } from "./assert.mjs";

/**
 * Read and parse a JSONL file. Returns an array of parsed JSON objects.
 * Blank lines are skipped. Returns an empty array if the file doesn't exist.
 */
export function readJsonl(path) {
	if (!existsSync(path)) {
		return [];
	}

	const raw = readFileSync(path, "utf-8").trim();
	if (!raw) {
		return [];
	}

	const lines = raw.split("\n").filter((l) => l.trim());
	const events = [];
	for (let i = 0; i < lines.length; i++) {
		try {
			events.push(JSON.parse(lines[i]));
		} catch {
			fail(
				`jsonl parse line ${i + 1}`,
				`invalid JSON: ${lines[i].slice(0, 80)}`,
			);
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
