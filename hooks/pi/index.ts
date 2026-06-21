/**
 * scryrs reference trace hook for Pi
 *
 * Transport-only observer that maps Pi tool_result events onto canonical
 * TraceEvent JSONL and delegates ingestion to `scryrs record --stdin`.
 *
 * Install: copy this directory into ~/.pi/agent/extensions/ or .pi/extensions/
 *
 * This hook never registers scryrs as an agent-callable tool, never modifies
 * agent-visible tool results, and fails open when scryrs is unavailable.
 */

// Local type stub — satisfied by @earendil-works/pi-coding-agent at Pi runtime.
// Uses explicit `any` for callback parameters whose full shape is defined by
// Pi's type system (see Pi documentation for event-specific payloads).
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

// — session-scoped identifier, generated once on extension load —
const SESSION_ID: string = crypto.randomUUID();

// — canonical schema version, kept in sync with scryrs-types SCHEMA_VERSION —
const SCHEMA_VERSION = "0.1.0";

// — the six Pi tools this hook tracks —
const TRACKED_TOOLS = new Set([
	"read",
	"bash",
	"ast_grep_search",
	"lsp_navigation",
	"edit",
	"write",
]);

// —— internals ——

/**
 * Collapse / replace embedded newlines in a string so the serialized JSON
 * occupies exactly one physical line. Kept consistent with the Claude Code
 * reference hook (collapseNewlines in scryrs-hook.mjs).
 */
function collapseNewlines(value: string): string {
	if (typeof value !== "string") return String(value ?? "");
	return value.replace(/\r?\n/g, " ⏎ ");
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
 * Pipe a pre-built TraceEvent envelope to scryrs record --stdin.
 * Fail-open: thrown errors and resolved non-zero exit codes are logged via
 * console.error, and the caller continues normally.  The subprocess is given
 * a 5 s timeout to prevent hung‑trace stalls.
 */
async function recordEvent(
	pi: ExtensionAPI,
	envelope: TraceEventEnvelope,
): Promise<void> {
	const line = JSON.stringify(envelope) + "\n";

	try {
		const result = await pi.exec("scryrs", ["record", "--stdin"], {
			input: line,
			timeout: 5000,
		});
		if (result?.code != null && result.code !== 0) {
			console.error(
				"scryrs trace hook: scryrs record exited non-zero " +
					`(code ${result.code}) — trace gap for this event.`,
			);
		}
	} catch (err: unknown) {
		console.error(
			"scryrs trace hook: failed to record event — scryrs may be missing, " +
				"timed-out, or rejected the event line.",
			err,
		);
	}
}

// —— public extension factory ——

export default function (pi: ExtensionAPI) {
	// SessionStart — emitted once per loaded session.
	pi.on("session_start", (_event: unknown, _ctx: unknown) => {
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

		if (!TRACKED_TOOLS.has(toolName)) {
			return undefined;
		}

		let eventType: string;
		let payload: Record<string, unknown>;

		// —— tool → TraceEvent mapping (design §D2-D4) ——

		switch (toolName) {
			case "read": {
				eventType = "FileOpened";
				const readPath: string | undefined = event.input?.path;
				if (readPath === undefined) {
					console.warn(
						"scryrs trace hook: read input missing 'path' field — using 'unknown'",
					);
				}
				payload = {
					type: "FileOpened",
					path: collapseNewlines(readPath ?? "unknown"),
				};
				break;
			}

			case "bash": {
				eventType = "CommandExecuted";
				const bashCmd: string | undefined = event.input?.command;
				if (bashCmd === undefined) {
					console.warn(
						"scryrs trace hook: bash input missing 'command' field — using 'unknown'",
					);
				}
				payload = {
					type: "CommandExecuted",
					command: collapseNewlines(bashCmd ?? "unknown"),
				};
				break;
			}

			case "ast_grep_search": {
				eventType = "SearchRun";
				const query: string | undefined = event.input?.query;
				if (query === undefined) {
					console.warn(
						"scryrs trace hook: ast_grep_search input missing 'query' field — using 'unknown'",
					);
				}
				payload = {
					type: "SearchRun",
					query: collapseNewlines(query ?? "unknown"),
				};
				break;
			}

			case "edit": {
				eventType = "EditMade";
				const editPath: string | undefined = event.input?.path;
				if (editPath === undefined) {
					console.warn(
						"scryrs trace hook: edit input missing 'path' field — using 'unknown'",
					);
				}
				payload = {
					type: "EditMade",
					target: collapseNewlines(editPath ?? "unknown"),
				};
				break;
			}

			case "write": {
				eventType = "EditMade";
				const writePath: string | undefined = event.input?.path;
				if (writePath === undefined) {
					console.warn(
						"scryrs trace hook: write input missing 'path' field — using 'unknown'",
					);
				}
				payload = {
					type: "EditMade",
					target: collapseNewlines(writePath ?? "unknown"),
				};
				break;
			}

			case "lsp_navigation": {
				const symbol: string | undefined = event.input?.symbol;
				if (symbol === undefined) {
					console.warn(
						"scryrs trace hook: lsp_navigation input missing 'symbol' field — using 'unknown'",
					);
				}
				const navTarget = symbol ?? "unknown";

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

		await recordEvent(pi, envelope);

		// Never modify the agent-visible tool result.
		return undefined;
	});
}
