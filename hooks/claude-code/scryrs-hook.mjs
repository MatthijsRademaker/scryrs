/**
 * scryrs Claude Code reference hook — PreToolUse transport.
 *
 * This module intercepts the nine scryrs-relevant Claude Code tools and
 * forwards canonical TraceEvent JSONL to `scryrs record --stdin`. It is a
 * thin observer — it never proxies tool execution, never rewrites stdout /
 * stderr / exit status, and always returns success to Claude Code.
 *
 * Fail-open: when scryrs is missing, exits non-zero, or crashes, a
 * timestamped warning is appended to `.scryrs/hooks/claude-code-warnings.log`
 * and the hook returns success to Claude Code so the original tool proceeds
 * normally.
 */

import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { appendFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SCHEMA_VERSION = "0.1.0";

/** Claude Code tools the hook intercepts. */
const WHITELIST = new Set([
	"read",
	"bash",
	"grep",
	"glob",
	"edit",
	"write",
	"notebookedit",
	"web_search",
	"web_fetch",
]);

/**
 * Tool-name → canonical TraceEvent type mapping.
 *
 * Each Claude Code tool maps to one scryrs event family.
 * Grep, Glob, and WebSearch all map to SearchRun.
 * Edit, Write, and NotebookEdit all map to EditMade.
 */
const TOOL_TO_EVENT_TYPE = {
	read: "FileOpened",
	bash: "CommandExecuted",
	grep: "SearchRun",
	glob: "SearchRun",
	edit: "EditMade",
	write: "EditMade",
	notebookedit: "EditMade",
	web_search: "SearchRun",
	web_fetch: "DocRetrieved",
};

// ---------------------------------------------------------------------------
// Session ID — per-process UUID v4
// ---------------------------------------------------------------------------

/**
 * Prefer a Claude Code-provided session identifier if set in the environment,
 * otherwise generate a UUID v4. This is evaluated once at module load so
 * every event within a single hook process shares the same session_id.
 */
function resolveSessionId() {
	const fromEnv =
		process.env.CLAUDE_SESSION_ID ||
		process.env.CC_SESSION_ID ||
		process.env.CLAUDE_CODE_SESSION_ID;
	if (fromEnv && fromEnv.length > 0) {
		return fromEnv;
	}
	// `randomUUID` is available in all Node.js versions that support ESM (≥ 14.17).
	return randomUUID();
}

const SESSION_ID = resolveSessionId();

// ---------------------------------------------------------------------------
// Warning log
// ---------------------------------------------------------------------------

const WARNING_LOG = resolve(
	process.cwd(),
	".scryrs/hooks/claude-code-warnings.log",
);

/**
 * Append a timestamped warning to the dedicated fail-open log file.
 * Creates parent directories if needed. Does NOT write to stdout or stderr
 * — warnings are out-of-band from the agent-visible tool context.
 */
function logWarning(message) {
	try {
		mkdirSync(dirname(WARNING_LOG), { recursive: true });
	} catch {
		// best-effort; a read-only filesystem should not crash the hook
	}
	const ts = new Date().toISOString();
	try {
		appendFileSync(WARNING_LOG, `${ts} ${message}\n`);
	} catch {
		// best-effort; missing directory permissions should not crash the hook
	}
}

// ---------------------------------------------------------------------------
// TraceEvent construction
// ---------------------------------------------------------------------------

/**
 * Return an RFC 3339 timestamp for *right now*.
 */
function rfc3339Now() {
	return new Date().toISOString();
}

/**
 * Extract the primary payload field from a Claude Code tool input.
 *
 * Claude Code provides `tool_input` as a structured object. The field we
 * extract depends on the tool:
 *
 * | Tool        | Source field          | Payload field |
 * |-------------|-----------------------|---------------|
 * | Read        | file_path             | path          |
 * | Bash        | command               | command       |
 * | Grep        | pattern               | query         |
 * | Glob        | pattern               | query         |
 * | Edit        | file_path             | target        |
 * | Write       | file_path             | target        |
 * | NotebookEdit| file_path             | target        |
 * | WebSearch   | searchTerm / query    | query         |
 * | WebFetch    | url / website         | doc_ref       |
 *
 * Returns the empty string if the expected field is missing.
 */
function extractPayloadValue(toolName, toolInput) {
	if (toolInput == null || typeof toolInput !== "object") {
		return "";
	}

	switch (toolName) {
		case "read":
			return String(toolInput.file_path ?? "");
		case "bash":
			return String(toolInput.command ?? "");
		case "grep":
			return String(toolInput.pattern ?? "");
		case "glob":
			return String(toolInput.pattern ?? "");
		case "edit":
			return String(toolInput.file_path ?? "");
		case "write":
			return String(toolInput.file_path ?? "");
		case "notebookedit":
			return String(toolInput.file_path ?? "");
		case "web_search":
			return String(toolInput.searchTerm ?? toolInput.query ?? "");
		case "web_fetch":
			return String(toolInput.url ?? toolInput.website ?? "");
		default:
			return "";
	}
}

/**
 * Collapse / replace embedded newlines in a string so the serialized JSON
 * occupies exactly one physical line.
 *
 * JSON.stringify would escape \n as \\n, but some consumers of JSONL expect
 * truly single-line records. We proactively replace literal newlines with
 * a visible marker to avoid downstream line-count confusion.
 */
function collapseNewlines(value) {
	if (typeof value !== "string") return value;
	return value.replace(/\r?\n/g, " ⏎ ");
}

/**
 * Build a canonical `TraceEvent` plain object matching the Rust
 * `crates/scryrs-types/src/lib.rs` schema.
 *
 * Every event carries:
 *   schema_version, timestamp, session_id, event_type, tool_name,
 *   self-describing payload (with `type` tag), and outcome.
 */
function buildTraceEvent(toolName, toolInput) {
	const eventType = TOOL_TO_EVENT_TYPE[toolName];
	const rawValue = extractPayloadValue(toolName, toolInput);
	const cleanValue = collapseNewlines(rawValue);

	// Build payload per the canonical schema. Every payload carries a `type`
	// tag matching its event_type so consumers can identify the concrete
	// shape from JSON alone.
	let payload;
	switch (eventType) {
		case "FileOpened":
			payload = { type: "FileOpened", path: cleanValue };
			break;
		case "CommandExecuted":
			payload = { type: "CommandExecuted", command: cleanValue };
			break;
		case "SearchRun":
			payload = { type: "SearchRun", query: cleanValue };
			break;
		case "EditMade":
			payload = { type: "EditMade", target: cleanValue };
			break;
		case "DocRetrieved":
			payload = { type: "DocRetrieved", doc_ref: cleanValue };
			break;
		default:
			// Should be unreachable — every whitelisted tool maps to a known type.
			payload = { type: eventType };
	}

	return {
		schema_version: SCHEMA_VERSION,
		timestamp: rfc3339Now(),
		session_id: SESSION_ID,
		event_type: eventType,
		tool_name: toolName,
		payload,
		outcome: { result: "Success" },
	};
}

// ---------------------------------------------------------------------------
// scryrs subprocess invocation
// ---------------------------------------------------------------------------

/**
 * Pipe a newline-delimited JSON TraceEvent to `scryrs record --stdin`.
 *
 * Returns a Promise that resolves with `true` on success or `false` when
 * scryrs is unavailable, exits non-zero, or crashes. The caller (the hook
 * entry point) always returns success to Claude Code regardless.
 *
 * Fail-open contract:
 *   - scryrs binary missing      → log warning, return false
 *   - subprocess spawn error     → log warning, return false
 *   - scryrs exits non-zero      → log warning, return false
 *   - stdin write error          → log warning, return false
 */
function forwardToScryrs(eventJson) {
	return new Promise((resolve) => {
		let settled = false;
		const done = (success, reason) => {
			if (settled) return;
			settled = true;
			if (!success && reason) logWarning(reason);
			resolve(success);
		};

		let child;
		try {
			child = spawn("scryrs", ["record", "--stdin"], {
				stdio: ["pipe", "ignore", "ignore"],
			});
		} catch (err) {
			done(false, `scryrs spawn error: ${err.message}`);
			return;
		}

		child.on("error", (err) => {
			const reason =
				err.code === "ENOENT"
					? "scryrs binary not found on PATH"
					: `scryrs process error: ${err.message}`;
			done(false, reason);
		});

		// Write the event as a single JSON line to scryrs stdin.
		// Wait for drain before ending to avoid truncation.
		const writeOk = child.stdin.write(eventJson + "\n");
		if (!writeOk) {
			child.stdin.once("drain", () => {
				child.stdin.end();
			});
		} else {
			child.stdin.end();
		}

		child.stdin.on("error", (err) => {
			done(false, `scryrs stdin write error: ${err.message}`);
		});

		child.on("exit", (code) => {
			if (code === 0) {
				done(true);
			} else {
				done(false, `scryrs record exited with code ${code}`);
			}
		});
	});
}

// ---------------------------------------------------------------------------
// Hook entry point — Claude Code PreToolUse hook contract
// ---------------------------------------------------------------------------

/**
 * Default export invoked by Claude Code before every tool execution.
 *
 * @param {object} input — Claude Code PreToolUse hook input
 * @param {string} input.tool_name   — Name of the tool about to execute
 * @param {object} input.tool_input  — Tool-specific input parameters
 * @returns {Promise<{continue: boolean}>} — Always `{continue: true}`
 *
 * Contract:
 *   - The hook never blocks tool execution. It always returns
 *     `{continue: true}` regardless of scryrs availability.
 *   - stdout / stderr of the original tool are never altered — the hook
 *     writes nothing to stdout or stderr by itself.
 *   - Fail-open: scryrs failures log to the dedicated warning file but
 *     never propagate to the agent.
 */
export default async function (input) {
	const toolName = (input?.tool_name ?? "").toLowerCase();

	// Pass through tools not in the whitelist — no trace event emitted.
	if (!WHITELIST.has(toolName)) {
		return { continue: true };
	}

	const toolInput = input?.tool_input ?? {};

	try {
		const event = buildTraceEvent(toolName, toolInput);
		const eventJson = JSON.stringify(event);

		// Forward to scryrs (fire-and-forget in terms of outcome — the hook
		// always returns success to Claude Code).
		await forwardToScryrs(eventJson);
	} catch (err) {
		// Catch-all: if JSON shaping itself throws (e.g., a circular reference
		// in tool input), log and continue. The hook must never crash Claude
		// Code's tool pipeline.
		logWarning(`scryrs hook internal error: ${err.message}`);
	}

	return { continue: true };
}
