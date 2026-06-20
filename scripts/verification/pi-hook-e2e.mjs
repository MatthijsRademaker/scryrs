/**
 * scryrs Pi end-to-end verification fixture.
 *
 * Installs `tsx` transiently in a temp directory, loads hooks/pi/index.ts
 * against a fake ExtensionAPI, and exercises SessionStart, tool capture,
 * failure propagation, and fail-open behavior against the real
 * `scryrs record --stdin` binary.
 *
 * Prerequisites:
 *   - Real `scryrs` binary on PATH (built via `cargo build --release`)
 *   - Working directory is the repository root
 *
 * Usage: node scripts/verification/pi-hook-e2e.mjs
 */

import { writeFileSync, mkdirSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { execSync, execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { pass, fail, summary } from "./lib/assert.mjs";
import { readJsonl, assertEventShape } from "./lib/jsonl.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..", "..");
const HOOK_SOURCE = join(ROOT, "hooks", "pi", "index.ts");

// -----------------------------------------------------------------------
// Temp directory helpers
// -----------------------------------------------------------------------
function tempDir() {
	return join(
		tmpdir(),
		`scryrs-pi-e2e-${Date.now()}-${Math.random().toString(36).slice(2)}`,
	);
}

// -----------------------------------------------------------------------
// Hook runner
function createHookRunner(dir, scryrsPath) {
	mkdirSync(dir, { recursive: true });
	execSync("npm install tsx", { cwd: dir, stdio: "pipe", timeout: 60000 });

	try {
		execFileSync(scryrsPath, ["init"], {
			cwd: dir,
			timeout: 5000,
			stdio: "ignore",
		});
	} catch {}

	return {
		/**
		 * Invoke the hook for a given event, wait for it to complete,
		 * and return the result.
		 */
		async runSessionStart(reason) {
			const scriptFile = join(dir, "invoke-session.mjs");
			// We write a self-contained script that loads the hook, fires the event, and exits.
			writeFileSync(
				scriptFile,
				`
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);

// The pi hook imports '@earendil-works/pi-coding-agent' as a type-only import.
// tsx strips type-only imports, so this is safe.
// The fake exec spawns real scryrs.
const { spawnSync } = require("node:child_process");

const fakeApi = {
  handlers: {},
  on(event, handler) {
    this.handlers[event] = handler;
  },
  async exec(command, args, options) {
    const input = options?.input ?? "";
    const timeout = options?.timeout ?? 5000;
    const cwd = options?.cwd ?? process.cwd();
    const result = spawnSync(command, args, {
      input,
      timeout,
      encoding: "utf-8",
      stdio: ["pipe", "pipe", "pipe"],
      cwd,
      env: { ...process.env },
    });
    return {
      stdout: result.stdout?.toString() || "",
      stderr: result.stderr?.toString() || "",
      code: result.status,
      killed: result.signal !== null,
    };
  },
};

// Dynamic import the hook
import(${JSON.stringify(HOOK_SOURCE)}).then((mod) => {
  mod.default(fakeApi);

  // Fire the requested event and wait
  const handler = fakeApi.handlers["session_start"];
  if (handler) {
    const result = handler(
      { reason: ${JSON.stringify(reason)}, previousSessionFile: undefined },
      {}
    );
    // Handler may return void or Promise — handle both
    if (result && typeof result.then === "function") {
      result.then(() => {
        // Give scryrs a moment to flush
        setTimeout(() => process.exit(0), 500);
      }).catch((err) => {
        console.error("HOOK_ERROR:", err.message);
        process.exit(1);
      });
    } else {
      // Handler returned void — give scryrs time then exit
      setTimeout(() => process.exit(0), 500);
    }
  } else {
    process.exit(0);
  }
}).catch((err) => {
  console.error("IMPORT_ERROR:", err.message);
  process.exit(1);
});
`,
			);

			const npx = join(dir, "node_modules", ".bin", "tsx");
			try {
				const stdout = execFileSync(npx, [scriptFile], {
					cwd: dir,
					timeout: 15000,
					encoding: "utf-8",
					stdio: ["ignore", "pipe", "pipe"],
				});
				return { ok: true, stdout: stdout?.toString() || "" };
			} catch (err) {
				return {
					ok: false,
					stderr: err.stderr?.toString() || "",
					stdout: err.stdout?.toString() || "",
				};
			}
		},

		/**
		 * Invoke the hook for a tool_result event.
		 */
		async runToolResult(toolName, input, isError = false) {
			const scriptFile = join(dir, "invoke.mjs");
			writeFileSync(
				scriptFile,
				`
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { spawnSync } = require("node:child_process");

const fakeApi = {
  handlers: {},
  on(event, handler) {
    this.handlers[event] = handler;
  },
  async exec(command, args, options) {
    const input = options?.input ?? "";
    const timeout = options?.timeout ?? 5000;
    const cwd = options?.cwd ?? process.cwd();
    const result = spawnSync(command, args, {
      input,
      timeout,
      encoding: "utf-8",
      stdio: ["pipe", "pipe", "pipe"],
      cwd,
      env: { ...process.env },
    });
    return {
      stdout: result.stdout?.toString() || "",
      stderr: result.stderr?.toString() || "",
      code: result.status,
      killed: result.signal !== null,
    };
  },
};

import(${JSON.stringify(HOOK_SOURCE)}).then((mod) => {
  mod.default(fakeApi);

  const handler = fakeApi.handlers["tool_result"];
  if (!handler) { process.exit(0); }

  const event = {
    toolName: ${JSON.stringify(toolName)},
    toolCallId: "call-test-123",
    input: ${JSON.stringify(input)},
    content: null,
    details: null,
    isError: ${isError},
  };

  const preSnapshot = JSON.stringify(event);

  handler(event, {}).then((result) => {
    // Output: result|preSnapshot|postSnapshot
    console.log("RESULT:" + JSON.stringify(result));
    console.log("PRESNAPSHOT:" + preSnapshot);
    console.log("POSTSNAPSHOT:" + JSON.stringify(event));
    setTimeout(() => process.exit(0), 500);
  }).catch((err) => {
    console.error("HOOK_ERROR:", err.message);
    process.exit(1);
  });
}).catch((err) => {
  console.error("IMPORT_ERROR:", err.message);
  process.exit(1);
});
`,
			);

			const npx = join(dir, "node_modules", ".bin", "tsx");
			try {
				const stdout = execFileSync(npx, [scriptFile], {
					cwd: dir,
					timeout: 15000,
					encoding: "utf-8",
					stdio: ["ignore", "pipe", "pipe"],
				});
				return { ok: true, stdout: stdout?.toString() || "" };
			} catch (err) {
				return {
					ok: false,
					stderr: err.stderr?.toString() || "",
					stdout: err.stdout?.toString() || "",
				};
			}
		},

		/**
		 * Run fail-open test: invoke tool_result with scryrs not on PATH.
		 */
		async runFailOpen(toolName, input) {
			const scriptFile = join(dir, "invoke-failopen.mjs");
			writeFileSync(
				scriptFile,
				`
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { spawnSync } = require("node:child_process");

const fakeApi = {
  handlers: {},
  on(event, handler) {
    this.handlers[event] = handler;
  },
  async exec(command, args, options) {
    const input = options?.input ?? "";
    const timeout = options?.timeout ?? 5000;
    const result = spawnSync(command, args, {
      input,
      timeout,
      encoding: "utf-8",
      stdio: ["pipe", "pipe", "pipe"],
      cwd: process.cwd(),
      env: { PATH: "/nonexistent", HOME: process.env.HOME },
    });
    // Simulate the error that happens when scryrs is not found
    if (result.error && result.error.code === "ENOENT") {
      throw new Error(\`Command not found: \${command}\`);
    }
    return {
      stdout: result.stdout?.toString() || "",
      stderr: result.stderr?.toString() || "",
      code: result.status,
      killed: result.signal !== null,
    };
  },
};

// Redirect console.error to capture fail-open messages
const errors = [];
const origError = console.error;
console.error = (...args) => { errors.push(args.join(" ")); };

import(${JSON.stringify(HOOK_SOURCE)}).then((mod) => {
  mod.default(fakeApi);

  const handler = fakeApi.handlers["tool_result"];
  if (!handler) { process.exit(0); }

  const event = {
    toolName: ${JSON.stringify(toolName)},
    toolCallId: "call-test-123",
    input: ${JSON.stringify(input)},
    content: null,
    details: null,
    isError: false,
  };

  const preSnapshot = JSON.stringify(event);

  handler(event, {}).then((result) => {
    console.log("RESULT:" + JSON.stringify(result));
    console.log("PRESNAPSHOT:" + preSnapshot);
    console.log("POSTSNAPSHOT:" + JSON.stringify(event));
    console.log("ERRORS:" + JSON.stringify(errors));
    setTimeout(() => process.exit(0), 500);
  }).catch((err) => {
    // Fail-open: should not crash
    console.error("UNEXPECTED_CRASH:", err.message);
    process.exit(1);
  });
}).catch((err) => {
  console.error("IMPORT_ERROR:", err.message);
  process.exit(1);
});
`,
			);

			const npx = join(dir, "node_modules", ".bin", "tsx");
			try {
				const stdout = execFileSync(npx, [scriptFile], {
					cwd: dir,
					timeout: 15000,
					encoding: "utf-8",
					stdio: ["ignore", "pipe", "pipe"],
				});
				return { ok: true, stdout: stdout?.toString() || "" };
			} catch (err) {
				return {
					ok: false,
					stderr: err.stderr?.toString() || "",
					stdout: err.stdout?.toString() || "",
				};
			}
		},
	};
}

// -----------------------------------------------------------------------
// Helper: parse fixture output lines
// -----------------------------------------------------------------------
function parseFixtureOutput(stdout) {
	const lines = stdout.split("\n").filter((l) => l.trim());
	const result = {};
	for (const line of lines) {
		if (line.startsWith("RESULT:")) {
			const val = line.slice("RESULT:".length);
			result.result = val === "undefined" ? undefined : JSON.parse(val);
		} else if (line.startsWith("PRESNAPSHOT:")) {
			result.preSnapshot = JSON.parse(line.slice("PRESNAPSHOT:".length));
		} else if (line.startsWith("POSTSNAPSHOT:")) {
			result.postSnapshot = JSON.parse(line.slice("POSTSNAPSHOT:".length));
		} else if (line.startsWith("ERRORS:")) {
			result.errors = JSON.parse(line.slice("ERRORS:".length));
		}
	}
	return result;
}

// -----------------------------------------------------------------------
// Test: Successful capture — SessionStart + six tracked tools
// -----------------------------------------------------------------------
async function testSuccessfulCapture() {
	console.log(`\n\x1b[33m--- Pi: Successful Capture ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = join(ROOT, "target", "release", "scryrs");
	const runner = createHookRunner(dir, scryrsPath);

	// 1. Emit SessionStart
	const ssResult = await runner.runSessionStart("manual-test");
	if (!ssResult.ok) {
		fail(
			"Pi SessionStart",
			`hook invocation failed: ${ssResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}
	pass("Pi SessionStart: hook invoked without error");

	// 2. Emit tool_result for each of the six tracked tools
	const tools = [
		{
			name: "read",
			input: { path: "/src/main.rs" },
			expectedType: "FileOpened",
			payloadKey: "path",
			payloadVal: "/src/main.rs",
		},
		{
			name: "bash",
			input: { command: "cargo build" },
			expectedType: "CommandExecuted",
			payloadKey: "command",
			payloadVal: "cargo build",
		},
		{
			name: "ast_grep_search",
			input: { query: "fn main" },
			expectedType: "SearchRun",
			payloadKey: "query",
			payloadVal: "fn main",
		},
		{
			name: "edit",
			input: { path: "/src/lib.rs" },
			expectedType: "EditMade",
			payloadKey: "target",
			payloadVal: "/src/lib.rs",
		},
		{
			name: "write",
			input: { path: "/src/new.rs" },
			expectedType: "EditMade",
			payloadKey: "target",
			payloadVal: "/src/new.rs",
		},
		{
			name: "lsp_navigation",
			input: { symbol: "MyStruct" },
			expectedType: "SymbolInspected",
			payloadKey: "name",
			payloadVal: "MyStruct",
		},
	];

	for (const tool of tools) {
		const trResult = await runner.runToolResult(tool.name, tool.input, false);
		if (!trResult.ok) {
			fail(
				`Pi ${tool.name}`,
				`hook invocation failed: ${trResult.stderr?.slice(0, 200)}`,
			);
			continue;
		}
		pass(`Pi ${tool.name}: hook invoked without error`);

		const parsed = parseFixtureOutput(trResult.stdout);
		if (parsed.result !== undefined) {
			fail(
				`Pi ${tool.name}: handler return`,
				`expected undefined, got ${JSON.stringify(parsed.result)}`,
			);
		} else {
			pass(`Pi ${tool.name}: handler returned undefined (non-interference)`);
		}
	}

	// 3. Assert events.jsonl
	const eventsJsonl = join(dir, ".scryrs", "events.jsonl");
	const events = readJsonl(eventsJsonl);

	// Should have SessionStart + 6 tool events = 7
	if (events.length < 7) {
		fail(
			"Pi events count",
			`expected at least 7 events (SessionStart + 6 tools), got ${events.length}`,
		);
	} else {
		pass(`Pi events count: ${events.length} events`);
	}

	// SessionStart
	const ssEvent = events.find((e) => e.event_type === "SessionStart");
	const sessionId = ssEvent?.session_id;
	if (ssEvent) {
		const shapeOk = assertEventShape(ssEvent, "SessionStart");
		if (
			shapeOk &&
			ssEvent.payload?.type === "SessionStart" &&
			ssEvent.outcome?.result === "Success"
		) {
			pass(
				"Pi SessionStart: correct envelope + payload.type + outcome.Success",
			);
		} else {
			fail(
				"Pi SessionStart",
				`shape or payload mismatch: ${JSON.stringify(ssEvent)}`,
			);
		}
	} else {
		fail("Pi SessionStart", "no SessionStart event found in events.jsonl");
	}

	// Check each tool event
	for (const tool of tools) {
		const toolEvent = events.find(
			(e) => e.event_type === tool.expectedType && e.tool_name === tool.name,
		);
		if (!toolEvent) {
			fail(
				`Pi ${tool.name}`,
				`no ${tool.expectedType} event for ${tool.name} found in events.jsonl`,
			);
			continue;
		}
		const shapeOk = assertEventShape(toolEvent, tool.expectedType, tool.name);
		if (shapeOk) {
			if (
				toolEvent.payload &&
				toolEvent.payload[tool.payloadKey] === tool.payloadVal
			) {
				pass(
					`Pi ${tool.name}: correct event type + payload.${tool.payloadKey}=${tool.payloadVal}`,
				);
			} else {
				fail(
					`Pi ${tool.name}: payload`,
					`payload.${tool.payloadKey}=${toolEvent.payload?.[tool.payloadKey]} expected=${tool.payloadVal}`,
				);
			}
		}
	}

	if (sessionId) {
		// Quick check: session_id should be a UUID-like string
		const uuidPattern =
			/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
		if (uuidPattern.test(sessionId)) {
			pass("Pi session_id: looks like a UUID v4");
		} else {
			fail("Pi session_id", `not a UUID v4: ${sessionId}`);
		}
	}

	// Check each tool event has a valid session_id
	for (const tool of tools) {
		const toolEvent = events.find(
			(e) => e.event_type === tool.expectedType && e.tool_name === tool.name,
		);
		if (toolEvent && toolEvent.session_id) {
			const uuidPattern =
				/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
			if (uuidPattern.test(toolEvent.session_id)) {
				pass(`Pi ${tool.name}: session_id is valid UUID`);
			} else {
				fail(
					`Pi ${tool.name}: session_id`,
					`not a UUID: ${toolEvent.session_id}`,
				);
			}
		}
	}

	rmSync(dir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Failure propagation — failing lsp_navigation
// -----------------------------------------------------------------------
async function testFailurePropagation() {
	console.log(`\n\x1b[33m--- Pi: Failure Propagation ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = join(ROOT, "target", "release", "scryrs");
	const runner = createHookRunner(dir, scryrsPath);

	const trResult = await runner.runToolResult(
		"lsp_navigation",
		{ symbol: "nonexistent_fn" },
		true, // isError
	);

	if (!trResult.ok) {
		fail(
			"Pi failure propagation",
			`hook invocation failed: ${trResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	const parsed = parseFixtureOutput(trResult.stdout);

	// Assert handler returns undefined (non-interference)
	if (parsed.result !== undefined) {
		fail(
			"Pi failure: handler return",
			`expected undefined, got ${JSON.stringify(parsed.result)}`,
		);
	} else {
		pass("Pi failure: handler returned undefined (non-interference)");
	}

	// Assert original payload unchanged (deep equality)
	if (parsed.preSnapshot && parsed.postSnapshot) {
		if (
			JSON.stringify(parsed.preSnapshot) === JSON.stringify(parsed.postSnapshot)
		) {
			pass("Pi failure: original error payload unchanged (deep equal)");
		} else {
			fail(
				"Pi failure: payload mutated",
				`pre=${parsed.preSnapshot} post=${parsed.postSnapshot}`,
			);
		}
	} else {
		fail("Pi failure: snapshot", "missing pre/post snapshot");
	}

	// Assert events.jsonl contains FailedLookup
	const eventsJsonl = join(dir, ".scryrs", "events.jsonl");
	const events = readJsonl(eventsJsonl);
	const failedEvent = events.find((e) => e.event_type === "FailedLookup");

	if (!failedEvent) {
		fail("Pi failure: FailedLookup", "no FailedLookup event in events.jsonl");
	} else {
		const shapeOk = assertEventShape(
			failedEvent,
			"FailedLookup",
			"lsp_navigation",
		);
		if (shapeOk) {
			pass("Pi failure: FailedLookup envelope correct");
		}

		// Check payload
		if (failedEvent.payload?.type === "FailedLookup") {
			pass("Pi failure: payload.type is FailedLookup");
		} else {
			fail(
				"Pi failure: payload.type",
				`expected FailedLookup, got ${failedEvent.payload?.type}`,
			);
		}

		if (failedEvent.payload?.subject === "nonexistent_fn") {
			pass("Pi failure: payload.subject is nonexistent_fn");
		} else {
			fail(
				"Pi failure: payload.subject",
				`expected nonexistent_fn, got ${failedEvent.payload?.subject}`,
			);
		}

		// Check outcome.result === "Failure"
		if (failedEvent.outcome?.result === "Failure") {
			pass("Pi failure: outcome.result is Failure");
		} else {
			fail(
				"Pi failure: outcome.result",
				`expected Failure, got ${failedEvent.outcome?.result}`,
			);
		}

		// Do NOT assert on outcome.reason — it's optional per the spec
	}

	rmSync(dir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Fail-open — scryrs missing
// -----------------------------------------------------------------------
async function testFailOpen() {
	console.log(`\n\x1b[33m--- Pi: Fail-open ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = join(ROOT, "target", "release", "scryrs");
	const runner = createHookRunner(dir, scryrsPath);

	const foResult = await runner.runFailOpen("read", { path: "/test.txt" });

	if (!foResult.ok) {
		// If the subprocess itself crashed, that's a fail-open violation
		fail("Pi fail-open", `subprocess crash: ${foResult.stderr?.slice(0, 200)}`);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	const parsed = parseFixtureOutput(foResult.stdout);

	// Handler should return undefined
	if (parsed.result !== undefined) {
		fail(
			"Pi fail-open: handler return",
			`expected undefined, got ${JSON.stringify(parsed.result)}`,
		);
	} else {
		pass("Pi fail-open: handler returned undefined (non-interference)");
	}

	// Original event should be unchanged
	if (parsed.preSnapshot && parsed.postSnapshot) {
		if (
			JSON.stringify(parsed.preSnapshot) === JSON.stringify(parsed.postSnapshot)
		) {
			pass("Pi fail-open: original payload unchanged");
		} else {
			fail("Pi fail-open: payload mutated");
		}
	}

	// console.error should have been called with a scryrs failure message
	if (parsed.errors && parsed.errors.length > 0) {
		pass(
			`Pi fail-open: console.error called with scryrs failure (${parsed.errors.length} message(s))`,
		);
	} else {
		fail(
			"Pi fail-open: console.error",
			"no console.error output — scryrs failure may not be logged",
		);
	}

	// Pass-through: event should not be persisted
	const eventsJsonl = join(dir, ".scryrs", "events.jsonl");
	if (existsSync(eventsJsonl)) {
		const events = readJsonl(eventsJsonl);
		if (events.length === 0) {
			pass("Pi fail-open: no events persisted (expected with scryrs missing)");
		}
	}

	rmSync(dir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Unlisted tools silently ignored
// -----------------------------------------------------------------------
async function testUnlistedTools() {
	console.log(`\n\x1b[33m--- Pi: Unlisted Tools ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = join(ROOT, "target", "release", "scryrs");
	const runner = createHookRunner(dir, scryrsPath);

	const trResult = await runner.runToolResult(
		"web_search",
		{ query: "test" },
		false,
	);

	if (!trResult.ok) {
		fail(
			"Pi unlisted: web_search",
			`hook invocation failed: ${trResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	const parsed = parseFixtureOutput(trResult.stdout);
	if (parsed.result !== undefined) {
		fail(
			"Pi unlisted: handler return",
			`expected undefined, got ${JSON.stringify(parsed.result)}`,
		);
	} else {
		pass("Pi unlisted: handler returned undefined");
	}

	// No event should be written for unlisted web_search
	const eventsJsonl = join(dir, ".scryrs", "events.jsonl");
	const events = readJsonl(eventsJsonl);
	const unlistedEvents = events.filter((e) => e.tool_name === "web_search");
	if (unlistedEvents.length === 0) {
		pass("Pi unlisted: no event persisted for unlisted tool");
	} else {
		fail(
			"Pi unlisted",
			`unlisted tool produced ${unlistedEvents.length} event(s)`,
		);
	}

	rmSync(dir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Main
// -----------------------------------------------------------------------
async function main() {
	await testSuccessfulCapture();
	await testFailurePropagation();
	await testFailOpen();
	await testUnlistedTools();

	summary();
}

main().catch((err) => {
	console.error("Pi E2E fixture crashed:", err);
	process.exit(2);
});
