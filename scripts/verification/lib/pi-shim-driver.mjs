/**
 * Pi shim driver — exercises a slimmed `hooks/pi/index.ts` transport shim with
 * a mock Pi runtime. Run under tsx so the TypeScript `index.ts` import is
 * transpiled (`node_modules/.bin/tsx scripts/verification/lib/pi-shim-driver.mjs`).
 *
 * Inputs (env):
 *   PI_HOOK_PATH  — absolute path to the installed/source index.ts
 *   SCRYRS_BIN    — scryrs binary to invoke for real persistence
 *   STORE_DIR     — cwd for the real `scryrs hook pi` invocation (store lands here)
 *   FAIL_OPEN     — "1" makes the mock exec return a non-zero code (fail-open test)
 *
 * Output (stdout): a single JSON line { calls: [{ cmd, args, content }], threw }.
 * The driver never throws on a delegation failure — that IS the fail-open contract.
 */

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const HOOK = process.env.PI_HOOK_PATH;
const SCRYRS_BIN = process.env.SCRYRS_BIN || "scryrs";
const STORE_DIR = process.env.STORE_DIR || process.cwd();
const FAIL_OPEN = process.env.FAIL_OPEN === "1";

const mod = await import(HOOK);
const hook = mod.default;

const calls = [];
const pi = {
	_handlers: {},
	on(event, handler) {
		this._handlers[event] = handler;
	},
	async exec(cmd, args) {
		const i = args.indexOf("--file");
		const file = i >= 0 ? args[i + 1] : null;
		let content = null;
		try {
			content = file ? readFileSync(file, "utf-8") : null;
		} catch {
			/* temp file already cleaned up or unreadable */
		}
		calls.push({ cmd, args, content });

		if (FAIL_OPEN) {
			return { stdout: "", stderr: "simulated failure", code: 1, killed: false };
		}
		// Replace the bare "scryrs" command with the resolved binary; the shim
		// always passes ["hook", "pi", "--file", <tmp>].
		const r = spawnSync(SCRYRS_BIN, args, { cwd: STORE_DIR, encoding: "utf-8" });
		return {
			stdout: r.stdout || "",
			stderr: r.stderr || "",
			code: r.status ?? 0,
			killed: false,
		};
	},
};

let threw = false;
try {
	hook(pi);
	// session_start resolves SESSION_ID synchronously (fire-and-forget exec).
	if (pi._handlers.session_start) {
		pi._handlers.session_start(
			{ reason: "startup" },
			{ sessionManager: { getSessionId: () => "pi-shim-session" } },
		);
	}
	// tool_result is awaited by the shim → its exec completes before we resolve.
	if (pi._handlers.tool_result) {
		await pi._handlers.tool_result({
			toolName: "read",
			input: { path: "README.md" },
			isError: false,
		});
	}
	// Let the fire-and-forget session_start exec settle.
	await new Promise((r) => setTimeout(r, 250));
} catch {
	threw = true;
}

console.log(JSON.stringify({ calls, threw }));
