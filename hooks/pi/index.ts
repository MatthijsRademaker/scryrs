/**
 * scryrs reference trace hook for Pi
 *
 * Transport-only observer that maps Pi tool_result events onto canonical
 * TraceEvent JSONL and delegates ingestion to `scryrs record --file <path>`.
 * Uses temp files because Pi's exec() opens stdin as /dev/null.
 *
 * Install: copy this directory into ~/.pi/agent/extensions/ or .pi/extensions/
 *
 * This hook never registers scryrs as an agent-callable tool, never modifies
 * agent-visible tool results, and fails open when scryrs is unavailable.
 */

import { mkdtempSync, writeFileSync, unlinkSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

// Local type stub — satisfied by @earendil-works/pi-coding-agent at Pi runtime.
interface ExtensionAPI {
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	on(event: string, handler: (...args: any[]) => void | Promise<void>): void;
	exec(
		command: string,
		args: string[],
		options?: Record<string, unknown>,
	): Promise<{
		stdout: string;
		stderr: string;
		code: number | null;
		killed: boolean;
	}>;
}

declare const process: {
	env: Record<string, string | undefined>;
	title?: string;
};

// — reusable temp directory, created once on extension load —
const TMP_DIR: string = (() => {
	try {
		return mkdtempSync(join(tmpdir(), "scryrs-pi-"));
	} catch {
		return "/tmp/scryrs-pi-fallback";
	}
})();

// — session-scoped identifier, resolved from Pi's SessionManager at session_start —
let SESSION_ID: string;

// — canonical schema version, kept in sync with scryrs-types SCHEMA_VERSION —
const SCHEMA_VERSION = "0.1.0";

const DEBUG_PREFIX = "[scryrs]";

const DEBUG_PREVIEW_LIMIT = 200;

type DebugMode = "off" | "debug" | "wire" | "raw";

// — the six Pi tools this hook tracks —
const TRACKED_TOOLS = new Set([
	"read",
	"bash",
	"ast_grep_search",
	"lsp_navigation",
	"edit",
	"write",
]);

const DEBUG_MODE = resolveDebugMode();

const DEBUG_ENABLED = DEBUG_MODE !== "off";

// —— internals ——

function resolveDebugMode(): DebugMode {
	const value = process.env.SCRYRS_DEBUG?.trim() ?? "";
	if (value === "") return "off";
	if (value === "wire") return "wire";
	if (value === "raw") return "raw";
	return "debug";
}

/**
 * Collapse / replace embedded newlines in a string so the serialized JSON
 * occupies exactly one physical line. Kept consistent with the Claude Code
 * reference hook (collapseNewlines in scryrs-hook.mjs).
 */
function collapseNewlines(value: string): string {
	if (typeof value !== "string") return String(value ?? "");
	return value.replace(/\r?\n/g, " ⏎ ");
}

function truncateDebug(value: string, maxLength = DEBUG_PREVIEW_LIMIT): string {
	if (value.length <= maxLength) return value;
	return `${value.slice(0, maxLength)}…(${value.length} chars)`;
}

function debugValue(value: unknown, maxLength = DEBUG_PREVIEW_LIMIT): string {
	if (typeof value === "string") {
		return truncateDebug(collapseNewlines(value), maxLength);
	}

	try {
		return truncateDebug(
			collapseNewlines(JSON.stringify(value) ?? String(value ?? "")),
			maxLength,
		);
	} catch {
		return truncateDebug(collapseNewlines(String(value ?? "")), maxLength);
	}
}

function asInputRecord(value: unknown): Record<string, unknown> | undefined {
	if (!value || typeof value !== "object" || Array.isArray(value)) {
		return undefined;
	}
	return value as Record<string, unknown>;
}

function inputKeys(input: unknown): string {
	const record = asInputRecord(input);
	if (!record) return "none";
	const keys = Object.keys(record).sort();
	return keys.length > 0 ? keys.join(",") : "none";
}

function wireInputPreview(input: unknown): string {
	const record = asInputRecord(input);
	if (!record) return "none";

	const preview = ["command", "path", "query", "symbol"]
		.filter((key) => record[key] !== undefined)
		.map((key) => `${key}:${debugValue(record[key], 120)}`);

	return preview.length > 0 ? preview.join(";") : "none";
}

function rawEventPreview(event: unknown): string {
	if (!event || typeof event !== "object") {
		return debugValue(event, 240);
	}

	const value = event as Record<string, unknown>;
	return debugValue(
		{
			toolName: value.toolName,
			toolCallId: value.toolCallId,
			input: value.input,
			content: value.content,
			details: value.details,
			isError: value.isError,
		},
		240,
	);
}

function debugLog(stage: string, fields: Record<string, unknown> = {}): void {
	if (!DEBUG_ENABLED) return;

	const parts = [`${DEBUG_PREFIX} stage=${stage}`];
	for (const [key, value] of Object.entries(fields)) {
		if (value === undefined) continue;
		parts.push(`${key}=${debugValue(value)}`);
	}

	console.error(parts.join(" "));
}

function debugStreamLines(
	stage: "record_stdout" | "record_stderr",
	value: string,
): void {
	if (!DEBUG_ENABLED || value.trim() === "") return;

	for (const line of value.split(/\r?\n/)) {
		if (line.trim() === "") continue;
		debugLog(stage, { preview: line });
	}
}

function mappedInputValue(
	toolName: string,
	input: Record<string, unknown> | undefined,
	fieldName: string,
): string {
	const value = input?.[fieldName];
	if (value === undefined) {
		console.warn(
			`scryrs trace hook: ${toolName} input missing '${fieldName}' field — using 'unknown'`,
		);
		debugLog("missing_field", {
			tool: toolName,
			wanted_field: fieldName,
			available_keys: inputKeys(input),
			fallback: "unknown",
		});
		return "unknown";
	}

	return collapseNewlines(String(value));
}

interface TraceEventEnvelope {
	schema_version: string;
	timestamp: string;
	session_id: string;
	event_type: string;
	tool_name?: string;
	payload: Record<string, unknown>;
	outcome: Record<string, unknown>;
}

/**
 * Write a pre-built TraceEvent envelope to a temp file and delegate
 * ingestion to `scryrs record --file <path>`.
 *
 * Pi's exec() uses stdio: ["ignore", "pipe", "pipe"] — stdin cannot be
 * written to, so we use a temp file instead of --stdin.
 *
 * Fail-open: thrown errors and resolved non-zero exit codes are logged via
 * console.error, and the caller continues normally.  The subprocess is given
 * a 5 s timeout to prevent hung‑trace stalls.
 */
async function recordEvent(
	pi: ExtensionAPI,
	envelope: TraceEventEnvelope,
): Promise<void> {
	const line = JSON.stringify(envelope) + "\n";
	const tmpFile = join(
		TMP_DIR,
		`${envelope.event_type}-${envelope.timestamp.replace(/[:.]/g, "-")}.jsonl`,
	);

	debugLog("record_send", {
		trace_event: envelope.event_type,
		tool: envelope.tool_name ?? "none",
		session_id: envelope.session_id,
		tmp_file: tmpFile,
	});

	try {
		writeFileSync(tmpFile, line, "utf-8");
	} catch (writeErr: unknown) {
		debugLog("record_write_error", {
			trace_event: envelope.event_type,
			tool: envelope.tool_name ?? "none",
			error: writeErr instanceof Error ? writeErr.message : String(writeErr),
		});
		console.error(
			"scryrs trace hook: failed to write temp file — trace gap for this event.",
			writeErr,
		);
		return;
	}

	try {
		const result = await pi.exec("scryrs", ["record", "--file", tmpFile], {
			timeout: 5000,
		});
		debugLog("record_result", {
			trace_event: envelope.event_type,
			tool: envelope.tool_name ?? "none",
			code: result?.code ?? "null",
			killed: result?.killed ?? false,
			stdout_preview: result?.stdout ?? "",
			stderr_preview: result?.stderr ?? "",
		});
		debugStreamLines("record_stdout", result?.stdout ?? "");
		debugStreamLines("record_stderr", result?.stderr ?? "");
		if (result?.code != null && result.code !== 0) {
			debugLog("record_nonzero", {
				trace_event: envelope.event_type,
				tool: envelope.tool_name ?? "none",
				code: result.code,
				killed: result.killed,
				stdout_preview: result.stdout,
				stderr_preview: result.stderr,
			});
			console.error(
				"scryrs trace hook: scryrs record exited non-zero " +
					`(code ${result.code}) — trace gap for this event.`,
			);
		}
	} catch (err: unknown) {
		debugLog("record_exec_error", {
			trace_event: envelope.event_type,
			tool: envelope.tool_name ?? "none",
			error: err instanceof Error ? err.message : String(err),
		});
		console.error(
			"scryrs trace hook: failed to record event — scryrs may be missing, " +
				"timed-out, or rejected the event line.",
			err,
		);
	} finally {
		try {
			unlinkSync(tmpFile);
		} catch {
			// best-effort cleanup
		}
	}
}

debugLog("hook_load", { debug_mode: DEBUG_MODE });

// —— public extension factory ——

export default function (pi: ExtensionAPI) {
	// SessionStart — emitted once per loaded session.
	pi.on("session_start", (event: unknown, ctx: unknown) => {
		// Resolve session_id from Pi's SessionManager, not our own UUID.
		const ctxAny = ctx as Record<string, unknown>;
		const sm = ctxAny?.sessionManager as
			| { getSessionId: () => string }
			| undefined;
		SESSION_ID = sm?.getSessionId() ?? crypto.randomUUID();

		debugLog("session_start", {
			reason:
				event && typeof event === "object" && "reason" in event
					? (event as { reason?: unknown }).reason
					: "unknown",
			trace_event: "SessionStart",
			session_id: SESSION_ID,
		});
		const envelope: TraceEventEnvelope = {
			schema_version: SCHEMA_VERSION,
			timestamp: new Date().toISOString(),
			session_id: SESSION_ID,
			event_type: "SessionStart",
			payload: { type: "SessionStart" },
			outcome: { result: "Success" },
		};

		// Fire-and-forget; never block session startup.
		recordEvent(pi, envelope);
	});

	// tool_result — post-execution observer for the six tracked tools.
	pi.on("tool_result", async (event: any) => {
		const toolName: string = event.toolName;
		const tracked = TRACKED_TOOLS.has(toolName);
		const input = asInputRecord(event.input);

		debugLog("tool_result", {
			tool: toolName,
			tracked,
			is_error: Boolean(event.isError),
			input_keys: inputKeys(input),
		});

		if (DEBUG_MODE === "wire" || DEBUG_MODE === "raw") {
			debugLog("tool_input_wire", {
				tool: toolName,
				preview: wireInputPreview(input),
			});
		}

		if (DEBUG_MODE === "raw") {
			debugLog("tool_input_raw", {
				tool: toolName,
				preview: rawEventPreview(event),
			});
		}

		if (!tracked) {
			return undefined;
		}

		let eventType: string;
		let payload: Record<string, unknown>;

		// —— tool → TraceEvent mapping (design §D2-D4) ——

		switch (toolName) {
			case "read": {
				eventType = "FileOpened";
				payload = {
					type: "FileOpened",
					path: mappedInputValue(toolName, input, "path"),
				};
				break;
			}

			case "bash": {
				eventType = "CommandExecuted";
				payload = {
					type: "CommandExecuted",
					command: mappedInputValue(toolName, input, "command"),
				};
				break;
			}

			case "ast_grep_search": {
				eventType = "SearchRun";
				payload = {
					type: "SearchRun",
					query: mappedInputValue(toolName, input, "query"),
				};
				break;
			}

			case "edit": {
				eventType = "EditMade";
				payload = {
					type: "EditMade",
					target: mappedInputValue(toolName, input, "path"),
				};
				break;
			}

			case "write": {
				eventType = "EditMade";
				payload = {
					type: "EditMade",
					target: mappedInputValue(toolName, input, "path"),
				};
				break;
			}

			case "lsp_navigation": {
				const navTarget = mappedInputValue(toolName, input, "symbol");

				if (event.isError) {
					eventType = "FailedLookup";
					payload = {
						type: "FailedLookup",
						subject: collapseNewlines(navTarget),
					};
				} else {
					eventType = "SymbolInspected";
					payload = {
						type: "SymbolInspected",
						name: collapseNewlines(navTarget),
					};
				}
				break;
			}

			default:
				// Unreachable — TRACKED_TOOLS guards above — but safe fallback.
				return undefined;
		}

		// —— outcome ——
		const outcome: Record<string, unknown> = event.isError
			? { result: "Failure", reason: "Tool execution error" }
			: { result: "Success" };

		// —— envelope ——
		const envelope: TraceEventEnvelope = {
			schema_version: SCHEMA_VERSION,
			timestamp: new Date().toISOString(),
			session_id: SESSION_ID,
			event_type: eventType,
			tool_name: toolName,
			payload,
			outcome,
		};

		debugLog("trace_mapped", {
			tool: toolName,
			trace_event: eventType,
			session_id: SESSION_ID,
		});

		await recordEvent(pi, envelope);

		// Never modify the agent-visible tool result.
		return undefined;
	});
}
