/**
 * scryrs Claude Code end-to-end verification fixture.
 *
 * Drives the NATIVE `scryrs hook claude-code` subcommand by piping a real
 * PreToolUse payload on stdin. There is no `.mjs` hook file and no `node` hook
 * process — translation lives in the Rust `scryrs-adapter-harness` crate and is
 * exercised through the shipped `scryrs` binary.
 *
 * Prerequisites:
 *   - Real `scryrs` binary (SCRYRS_BIN env, or target/release/scryrs)
 *
 * Usage: node scripts/verification/claude-code-e2e.mjs
 */

import { mkdtempSync, mkdirSync, rmSync, existsSync, readFileSync } from "node:fs";
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

function freshDir(prefix) {
	return mkdtempSync(join(tmpdir(), prefix));
}

/**
 * Pipe a PreToolUse payload to `scryrs hook claude-code` on stdin.
 * Returns { status, stdout, stderr }.
 */
function runHook(payload, { cwd, env = {} } = {}) {
	const result = spawnSync(SCRYRS_BIN, ["hook", "claude-code"], {
		input: typeof payload === "string" ? payload : JSON.stringify(payload),
		cwd: cwd || ROOT,
		env: { ...process.env, ...env },
		encoding: "utf-8",
		timeout: 15000,
	});
	return {
		status: result.status,
		stdout: result.stdout ?? "",
		stderr: result.stderr ?? "",
	};
}

function storeFor(dir) {
	return join(dir, ".scryrs", "scryrs.db");
}

function readWarningLog(dir) {
	const p = join(dir, ".scryrs", "hooks", "claude-code-warnings.log");
	return existsSync(p) ? readFileSync(p, "utf-8") : "";
}

// -----------------------------------------------------------------------
// Test: tracked tools map to canonical events and persist under payload cwd
// -----------------------------------------------------------------------
function testTrackedTools() {
	console.log("\n\x1b[33m--- Claude Code: tracked tools (native command) ---\x1b[0m");

	const cases = [
		{ tool: "Read", input: { file_path: "/src/main.rs" }, type: "FileOpened" },
		{ tool: "Grep", input: { pattern: "fn main" }, type: "SearchRun" },
		{ tool: "Glob", input: { pattern: "**/*.rs" }, type: "SearchRun" },
		{ tool: "WebSearch", input: { query: "rust traits" }, type: "SearchRun" },
		{ tool: "Edit", input: { file_path: "/src/a.rs" }, type: "EditMade" },
		{ tool: "Write", input: { file_path: "/src/b.rs" }, type: "EditMade" },
		{ tool: "NotebookEdit", input: { notebook_path: "/n.ipynb" }, type: "EditMade" },
		{ tool: "WebFetch", input: { url: "https://example.com" }, type: "DocRetrieved" },
	];

	for (const c of cases) {
		const dir = freshDir("scryrs-cc-");
		try {
			const r = runHook(
				{ session_id: "abc123", cwd: dir, tool_name: c.tool, tool_input: c.input },
				{ cwd: ROOT },
			);
			if (r.status !== 0) {
				fail(`${c.tool}: exit 0`, `got ${r.status}, stderr=${r.stderr}`);
				continue;
			}
			if (r.stdout !== "") {
				fail(`${c.tool}: empty stdout`, `got: ${JSON.stringify(r.stdout)}`);
				continue;
			}
			pass(`${c.tool}: exit 0 with empty stdout`);

			// Event persisted under the payload cwd (D5), not under ROOT.
			const events = readEventsDb(storeFor(dir));
			if (events.length !== 1) {
				fail(`${c.tool}: exactly one event under payload cwd`, `got ${events.length}`);
				continue;
			}
			pass(`${c.tool}: one event persisted under payload cwd`);
			if (assertEventShape(events[0], c.type, c.tool)) {
				pass(`${c.tool}: canonical ${c.type} envelope`);
			}
			if (events[0].session_id === "abc123") {
				pass(`${c.tool}: session_id from payload`);
			} else {
				fail(`${c.tool}: session_id from payload`, `got ${events[0].session_id}`);
			}
			if (events[0].outcome.result === "Success") {
				pass(`${c.tool}: pre-exec outcome is Success`);
			} else {
				fail(`${c.tool}: outcome Success`, `got ${events[0].outcome.result}`);
			}
		} finally {
			rmSync(dir, { recursive: true, force: true });
		}
	}
}

// -----------------------------------------------------------------------
// Test: store resolves against payload cwd, not the process cwd
// -----------------------------------------------------------------------
function testStoreResolvesToPayloadCwd() {
	console.log("\n\x1b[33m--- Claude Code: store resolves to payload cwd ---\x1b[0m");
	const procDir = freshDir("scryrs-cc-proc-");
	const payloadDir = freshDir("scryrs-cc-payload-");
	try {
		const r = runHook(
			{
				session_id: "s1",
				cwd: payloadDir,
				tool_name: "Read",
				tool_input: { file_path: "/x.rs" },
			},
			{ cwd: procDir },
		);
		const inPayload = readEventsDb(storeFor(payloadDir)).length;
		const inProc = readEventsDb(storeFor(procDir)).length;
		if (r.status === 0 && inPayload === 1 && inProc === 0) {
			pass("event written under payload cwd, not process cwd");
		} else {
			fail(
				"store resolves to payload cwd",
				`payload=${inPayload} proc=${inProc} status=${r.status}`,
			);
		}
	} finally {
		rmSync(procDir, { recursive: true, force: true });
		rmSync(payloadDir, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// Test: untracked tool persists nothing
// -----------------------------------------------------------------------
function testUntracked() {
	console.log("\n\x1b[33m--- Claude Code: untracked tool pass-through ---\x1b[0m");
	const dir = freshDir("scryrs-cc-untracked-");
	try {
		const r = runHook(
			{ session_id: "s1", cwd: dir, tool_name: "TodoWrite", tool_input: {} },
			{ cwd: ROOT },
		);
		const count = readEventsDb(storeFor(dir)).length;
		if (r.status === 0 && count === 0) {
			pass("untracked tool: exit 0 and no event persisted");
		} else {
			fail("untracked tool pass-through", `status=${r.status} count=${count}`);
		}
	} finally {
		rmSync(dir, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// Test: Bash is debug-gated
// -----------------------------------------------------------------------
function testBashDebugGating() {
	console.log("\n\x1b[33m--- Claude Code: Bash debug-gating ---\x1b[0m");

	// Without SCRYRS_DEBUG → dropped.
	const dirOff = freshDir("scryrs-cc-bash-off-");
	try {
		runHook(
			{ session_id: "s1", cwd: dirOff, tool_name: "Bash", tool_input: { command: "ls" } },
			{ cwd: ROOT, env: { SCRYRS_DEBUG: "" } },
		);
		const count = readEventsDb(storeFor(dirOff)).length;
		if (count === 0) pass("Bash dropped when SCRYRS_DEBUG unset");
		else fail("Bash debug-gating (off)", `got ${count} events`);
	} finally {
		rmSync(dirOff, { recursive: true, force: true });
	}

	// With SCRYRS_DEBUG → CommandExecuted.
	const dirOn = freshDir("scryrs-cc-bash-on-");
	try {
		runHook(
			{ session_id: "s1", cwd: dirOn, tool_name: "Bash", tool_input: { command: "cargo build" } },
			{ cwd: ROOT, env: { SCRYRS_DEBUG: "1" } },
		);
		const events = readEventsDb(storeFor(dirOn));
		if (events.length === 1 && events[0].event_type === "CommandExecuted") {
			pass("Bash captured as CommandExecuted when SCRYRS_DEBUG set");
		} else {
			fail("Bash debug-gating (on)", `got ${JSON.stringify(events)}`);
		}
	} finally {
		rmSync(dirOn, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// Test: fail-open — malformed stdin and unwritable store both exit 0 + log
// -----------------------------------------------------------------------
function testFailOpen() {
	console.log("\n\x1b[33m--- Claude Code: fail-open ---\x1b[0m");

	// Malformed JSON on stdin.
	const dirBad = freshDir("scryrs-cc-malformed-");
	try {
		const r = spawnSync(SCRYRS_BIN, ["hook", "claude-code"], {
			input: "this is not json",
			cwd: dirBad,
			env: { ...process.env },
			encoding: "utf-8",
			timeout: 15000,
		});
		const log = readWarningLog(dirBad);
		if (r.status === 0 && (r.stdout ?? "") === "" && log.includes("malformed JSON")) {
			pass("malformed stdin: exit 0, empty stdout, warning logged");
		} else {
			fail(
				"malformed stdin fail-open",
				`status=${r.status} stdout=${JSON.stringify(r.stdout)} log=${JSON.stringify(log)}`,
			);
		}
	} finally {
		rmSync(dirBad, { recursive: true, force: true });
	}

	// Unwritable store: the warning-log dir (payload cwd) stays writable, but
	// the db path is blocked by pre-creating <cwd>/.scryrs/scryrs.db AS A
	// DIRECTORY so EventStore::open cannot create the database file there.
	const cwdDir = freshDir("scryrs-cc-failopen-cwd-");
	try {
		mkdirSync(join(cwdDir, ".scryrs", "scryrs.db"), { recursive: true });
		const r = runHook(
			{ session_id: "s1", cwd: cwdDir, tool_name: "Read", tool_input: { file_path: "/a.rs" } },
			{ cwd: ROOT },
		);
		const log = readWarningLog(cwdDir);
		if (
			r.status === 0 &&
			(log.includes("cannot open store") || log.includes("cannot append"))
		) {
			pass("unwritable store: exit 0 and warning logged");
		} else {
			fail("unwritable store fail-open", `status=${r.status} log=${JSON.stringify(log)}`);
		}
	} finally {
		rmSync(cwdDir, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// Main
// -----------------------------------------------------------------------
console.log("scryrs Claude Code E2E — native `scryrs hook claude-code`");
console.log(`Binary: ${SCRYRS_BIN}`);

testTrackedTools();
testStoreResolvesToPayloadCwd();
testUntracked();
testBashDebugGating();
testFailOpen();
summary();
