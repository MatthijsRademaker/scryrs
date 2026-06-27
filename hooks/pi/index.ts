/**
 * scryrs reference trace hook for Pi — transport-only shim.
 *
 * Pi loads an in-process extension module (there is no subprocess hook for Pi
 * the way Claude Code has), so this file cannot be deleted. It is reduced to a
 * pure transport: it registers `session_start` and `tool_result`, resolves the
 * `session_id` from Pi's SessionManager, serializes the RAW harness event to a
 * temp file, and hands it to `scryrs hook pi --file <tmp>`.
 *
 * ALL tool→TraceEvent translation lives in the Rust `scryrs-adapter-harness`
 * crate. This shim contains no mapping switch, no whitelist, and no schema
 * knowledge — it just forwards events and decides nothing.
 *
 * A temp file is used because Pi's exec() opens stdin as /dev/null, so the
 * event cannot be piped on stdin.
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
	cwd(): string;
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
let SESSION_ID = "";

const DEBUG_ENABLED = (process.env.SCRYRS_DEBUG ?? "").trim() !== "";

function debug(message: string): void {
	if (DEBUG_ENABLED) console.error(`[scryrs] ${message}`);
}

let TMP_SEQ = 0;

/**
 * Write the raw harness event (with the resolved session_id injected) to a temp
 * file and delegate to `scryrs hook pi --file <tmp>`.
 *
 * Fail-open: write errors, exec errors, and non-zero exits are logged via
 * console.error and never alter the agent-visible tool result. (In practice
 * `scryrs hook` always exits 0; this guard covers a missing/old binary.)
 */
async function forwardRawEvent(
	pi: ExtensionAPI,
	rawEvent: Record<string, unknown>,
): Promise<void> {
	const payload = JSON.stringify({
		...rawEvent,
		session_id: SESSION_ID,
		cwd: process.cwd(),
	});
	const tmpFile = join(TMP_DIR, `event-${TMP_SEQ++}.json`);

	try {
		writeFileSync(tmpFile, payload, "utf-8");
	} catch (writeErr: unknown) {
		console.error(
			"scryrs trace hook: failed to write temp file — trace gap for this event.",
			writeErr,
		);
		return;
	}

	try {
		const result = await pi.exec("scryrs", ["hook", "pi", "--file", tmpFile], {
			timeout: 5000,
		});
		if (result?.code != null && result.code !== 0) {
			console.error(
				`scryrs trace hook: scryrs hook pi exited non-zero (code ${result.code}) — trace gap for this event.`,
			);
		}
	} catch (err: unknown) {
		console.error(
			"scryrs trace hook: failed to forward event — scryrs may be missing or timed out.",
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

debug("hook_load");

// —— public extension factory ——

export default function (pi: ExtensionAPI) {
	// SessionStart — resolve the session id and forward a lifecycle marker.
	pi.on("session_start", (event: unknown, ctx: unknown) => {
		const ctxAny = ctx as Record<string, unknown>;
		const sm = ctxAny?.sessionManager as
			| { getSessionId: () => string }
			| undefined;
		SESSION_ID = sm?.getSessionId() ?? crypto.randomUUID();

		const reason =
			event && typeof event === "object" && "reason" in event
				? (event as { reason?: unknown }).reason
				: undefined;

		debug(`session_start session_id=${SESSION_ID}`);

		// No toolName → the Rust pi adapter maps this to SessionStart.
		// Fire-and-forget; never block session startup.
		forwardRawEvent(pi, { reason });
	});

	// tool_result — forward the raw event; the Rust adapter decides everything.
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	pi.on("tool_result", async (event: any) => {
		debug(
			`tool_result tool=${event?.toolName} is_error=${Boolean(event?.isError)}`,
		);

		await forwardRawEvent(pi, {
			toolName: event?.toolName,
			input: event?.input,
			isError: Boolean(event?.isError),
		});

		// Never modify the agent-visible tool result.
		return undefined;
	});
}
