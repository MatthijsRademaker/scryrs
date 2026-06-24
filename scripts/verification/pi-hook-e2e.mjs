/**
 * scryrs Pi end-to-end verification fixture.
 *
 * Two layers, both against the real `scryrs` binary:
 *
 *  A. Drives the NATIVE `scryrs hook pi --file <tmp>` subcommand with crafted
 *     raw Pi events and asserts the canonical mapping (including isError→Failure
 *     and the lsp_navigation SymbolInspected/FailedLookup branches). Translation
 *     lives in the Rust `scryrs-adapter-harness` crate.
 *
 *  B. Loads the slimmed transport shim `hooks/pi/index.ts` via tsx with a mock
 *     Pi runtime, proving it forwards the raw event to `scryrs hook pi --file`
 *     and fails open when that invocation errors.
 *
 * Prerequisites:
 *   - Real `scryrs` binary (SCRYRS_BIN env, or target/release/scryrs)
 *   - tsx and better-sqlite3 available (npm install)
 *
 * Usage: node scripts/verification/pi-hook-e2e.mjs
 */

import {
	mkdtempSync,
	rmSync,
	writeFileSync,
	existsSync,
	readFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { pass, fail, summary } from "./lib/assert.mjs";
import { readEventsDb, assertEventShape } from "./lib/db.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..", "..");
const SCRYRS_BIN =
	process.env.SCRYRS_BIN || join(ROOT, "target", "release", "scryrs");
const HOOK_TS = join(ROOT, "hooks", "pi", "index.ts");
const TSX = join(ROOT, "node_modules", ".bin", "tsx");
const SHIM_DRIVER = join(__dirname, "lib", "pi-shim-driver.mjs");

function freshDir(prefix) {
	return mkdtempSync(join(tmpdir(), prefix));
}

function storeFor(dir) {
	return join(dir, ".scryrs", "scryrs.db");
}

/**
 * Write a raw Pi event to a temp file and run `scryrs hook pi --file <tmp>`
 * with cwd = `dir` (store resolves under `dir`).
 */
function runPiFile(dir, rawEvent, env = {}) {
	const tmp = join(dir, "event.json");
	writeFileSync(tmp, JSON.stringify(rawEvent));
	const result = spawnSync(SCRYRS_BIN, ["hook", "pi", "--file", tmp], {
		cwd: dir,
		env: { ...process.env, ...env },
		encoding: "utf-8",
		timeout: 15000,
	});
	return { status: result.status, stdout: result.stdout ?? "" };
}

// -----------------------------------------------------------------------
// A. Native `scryrs hook pi --file` mapping
// -----------------------------------------------------------------------
function testNativeMapping() {
	console.log("\n\x1b[33m--- Pi: native `scryrs hook pi --file` mapping ---\x1b[0m");

	const cases = [
		{
			name: "read",
			raw: { session_id: "pi-1", toolName: "read", input: { path: "src/a.rs" }, isError: false },
			type: "FileOpened",
			tool: "read",
			outcome: "Success",
		},
		{
			name: "ast_grep_search",
			raw: { session_id: "pi-1", toolName: "ast_grep_search", input: { query: "fn main" }, isError: false },
			type: "SearchRun",
			tool: "ast_grep_search",
			outcome: "Success",
		},
		{
			name: "edit",
			raw: { session_id: "pi-1", toolName: "edit", input: { path: "src/a.rs" }, isError: false },
			type: "EditMade",
			tool: "edit",
			outcome: "Success",
		},
		{
			name: "write",
			raw: { session_id: "pi-1", toolName: "write", input: { path: "src/b.rs" }, isError: false },
			type: "EditMade",
			tool: "write",
			outcome: "Success",
		},
		{
			name: "lsp_navigation success",
			raw: { session_id: "pi-1", toolName: "lsp_navigation", input: { symbol: "Dispatcher" }, isError: false },
			type: "SymbolInspected",
			tool: "lsp_navigation",
			outcome: "Success",
		},
		{
			name: "lsp_navigation error",
			raw: { session_id: "pi-1", toolName: "lsp_navigation", input: { symbol: "Missing" }, isError: true },
			type: "FailedLookup",
			tool: "lsp_navigation",
			outcome: "Failure",
		},
		{
			name: "read isError",
			raw: { session_id: "pi-1", toolName: "read", input: { path: "x.rs" }, isError: true },
			type: "FileOpened",
			tool: "read",
			outcome: "Failure",
		},
		{
			name: "session_start",
			raw: { session_id: "pi-1", reason: "startup" },
			type: "SessionStart",
			tool: undefined,
			outcome: "Success",
		},
	];

	for (const c of cases) {
		const dir = freshDir("scryrs-pi-");
		try {
			const r = runPiFile(dir, c.raw);
			if (r.status !== 0 || r.stdout !== "") {
				fail(`${c.name}: exit 0 empty stdout`, `status=${r.status} stdout=${JSON.stringify(r.stdout)}`);
				continue;
			}
			const events = readEventsDb(storeFor(dir));
			if (events.length !== 1) {
				fail(`${c.name}: exactly one event`, `got ${events.length}`);
				continue;
			}
			if (assertEventShape(events[0], c.type, c.tool)) {
				pass(`${c.name}: ${c.type}`);
			}
			if (events[0].outcome.result === c.outcome) {
				pass(`${c.name}: outcome ${c.outcome}`);
			} else {
				fail(`${c.name}: outcome ${c.outcome}`, `got ${events[0].outcome.result}`);
			}
		} finally {
			rmSync(dir, { recursive: true, force: true });
		}
	}
}

function testUntracked() {
	console.log("\n\x1b[33m--- Pi: untracked tool pass-through ---\x1b[0m");
	const dir = freshDir("scryrs-pi-untracked-");
	try {
		const r = runPiFile(dir, { session_id: "pi-1", toolName: "todo", input: {}, isError: false });
		const count = readEventsDb(storeFor(dir)).length;
		if (r.status === 0 && count === 0) pass("untracked Pi tool: exit 0, no event");
		else fail("untracked Pi tool", `status=${r.status} count=${count}`);
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
}

function testBashGating() {
	console.log("\n\x1b[33m--- Pi: Bash debug-gating ---\x1b[0m");
	const off = freshDir("scryrs-pi-bash-off-");
	try {
		runPiFile(off, { session_id: "pi-1", toolName: "bash", input: { command: "ls" }, isError: false }, { SCRYRS_DEBUG: "" });
		const count = readEventsDb(storeFor(off)).length;
		if (count === 0) pass("Pi bash dropped when SCRYRS_DEBUG unset");
		else fail("Pi bash gating (off)", `got ${count}`);
	} finally {
		rmSync(off, { recursive: true, force: true });
	}
	const on = freshDir("scryrs-pi-bash-on-");
	try {
		runPiFile(on, { session_id: "pi-1", toolName: "bash", input: { command: "cargo test" }, isError: false }, { SCRYRS_DEBUG: "1" });
		const events = readEventsDb(storeFor(on));
		if (events.length === 1 && events[0].event_type === "CommandExecuted") {
			pass("Pi bash captured as CommandExecuted when SCRYRS_DEBUG set");
		} else {
			fail("Pi bash gating (on)", `got ${JSON.stringify(events)}`);
		}
	} finally {
		rmSync(on, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// B. The slimmed index.ts shim delegates to `scryrs hook pi --file`
// -----------------------------------------------------------------------
function runShimDriver(storeDir, { failOpen = false } = {}) {
	const result = spawnSync(TSX, [SHIM_DRIVER], {
		cwd: ROOT,
		env: {
			...process.env,
			PI_HOOK_PATH: HOOK_TS,
			SCRYRS_BIN,
			STORE_DIR: storeDir,
			FAIL_OPEN: failOpen ? "1" : "0",
		},
		encoding: "utf-8",
		timeout: 30000,
	});
	let parsed = null;
	try {
		parsed = JSON.parse((result.stdout ?? "").trim().split("\n").pop());
	} catch {
		/* leave null */
	}
	return { status: result.status, parsed, stderr: result.stderr ?? "" };
}

function testShimDelegation() {
	console.log("\n\x1b[33m--- Pi: shim delegates to native command ---\x1b[0m");
	if (!existsSync(TSX)) {
		fail("tsx available", `not found at ${TSX} — run npm install`);
		return;
	}
	const dir = freshDir("scryrs-pi-shim-");
	try {
		writeFileSync(join(dir, "README.md"), "# fixture\n");
		const { status, parsed } = runShimDriver(dir);
		if (status !== 0 || !parsed) {
			fail("shim driver runs", `status=${status}`);
			return;
		}
		const toolCall = (parsed.calls || []).find((c) => {
			try {
				return JSON.parse(c.content)?.toolName === "read";
			} catch {
				return false;
			}
		});
		if (
			toolCall &&
			toolCall.cmd === "scryrs" &&
			toolCall.args[0] === "hook" &&
			toolCall.args[1] === "pi" &&
			toolCall.args[2] === "--file"
		) {
			pass("shim invokes `scryrs hook pi --file <tmp>`");
		} else {
			fail("shim delegates to native command", JSON.stringify(parsed.calls));
			return;
		}
		// Forwarded raw event carries the resolved session_id.
		try {
			const fwd = JSON.parse(toolCall.content);
			if (fwd.session_id === "pi-shim-session" && fwd.input?.path === "README.md") {
				pass("shim forwards raw event with injected session_id");
			} else {
				fail("shim forwards raw event", JSON.stringify(fwd));
			}
		} catch {
			fail("shim forwards raw event", "unparseable forwarded content");
		}
		// Real persistence through the delegated command.
		const events = readEventsDb(storeFor(dir));
		if (events.some((e) => e.event_type === "FileOpened" && e.tool_name === "read")) {
			pass("shim delegation persists a FileOpened event");
		} else {
			fail("shim delegation persists event", JSON.stringify(events));
		}
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
}

function testShimFailOpen() {
	console.log("\n\x1b[33m--- Pi: shim fails open on delegation failure ---\x1b[0m");
	if (!existsSync(TSX)) {
		fail("tsx available", `not found at ${TSX} — run npm install`);
		return;
	}
	const dir = freshDir("scryrs-pi-failopen-");
	try {
		writeFileSync(join(dir, "README.md"), "# fixture\n");
		const { status, parsed } = runShimDriver(dir, { failOpen: true });
		// The shim must not throw even though `scryrs hook pi` "failed".
		if (status === 0 && parsed && parsed.threw === false) {
			pass("shim does not throw when delegation returns non-zero");
		} else {
			fail("shim fail-open", `status=${status} parsed=${JSON.stringify(parsed)}`);
		}
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// C. The shim contains no translation logic (single source of truth in Rust)
// -----------------------------------------------------------------------
function testShimHasNoMapping() {
	console.log("\n\x1b[33m--- Pi: shim contains no tool→event mapping ---\x1b[0m");
	const src = readFileSync(HOOK_TS, "utf-8");
	const checks = [
		[!src.includes("scryrs record"), "shim does not call `scryrs record`"],
		[src.includes("hook"), "shim references the native `scryrs hook pi` command"],
		[!src.includes("TRACKED_TOOLS"), "shim has no TRACKED_TOOLS whitelist"],
		[!src.includes("FileOpened"), "shim has no event-type mapping (no FileOpened)"],
		[!src.includes("SymbolInspected"), "shim has no event-type mapping (no SymbolInspected)"],
	];
	for (const [ok, name] of checks) {
		if (ok) pass(name);
		else fail(name, "unexpected content in hooks/pi/index.ts");
	}
}

console.log("scryrs Pi E2E — native `scryrs hook pi` + transport shim");
console.log(`Binary: ${SCRYRS_BIN}`);

testNativeMapping();
testUntracked();
testBashGating();
testShimDelegation();
testShimFailOpen();
testShimHasNoMapping();
summary();
