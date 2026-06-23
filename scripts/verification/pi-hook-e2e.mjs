/**
 * scryrs Pi end-to-end verification fixture.
 *
 * Installs `tsx` transiently in a temp directory, loads hooks/pi/index.ts
 * against a fake ExtensionAPI whose exec() matches Pi's real semantics
 * (`stdio: ["ignore", "pipe", "pipe"]`), and exercises SessionStart,
 * tool capture, failure propagation, and fail-open behavior against the real
 * `scryrs record --file <PATH>` binary path.
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
import { execSync, execFileSync, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { pass, fail, summary } from "./lib/assert.mjs";
import { readEventsDb, assertEventShape } from "./lib/db.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..", "..");
const HOOK_SOURCE = join(ROOT, "hooks", "pi", "index.ts");
const SCRYRS_BIN = process.env.SCRYRS_BIN || join(ROOT, "target", "release", "scryrs");

// -----------------------------------------------------------------------
// Wait-for-events helper — polls scryrs.db until expected count or timeout
// -----------------------------------------------------------------------
function waitForEventCount(dbPath, expectedCount, timeoutMs = 5000) {
	const start = Date.now();
	while (Date.now() - start < timeoutMs) {
		if (existsSync(dbPath)) {
			const events = readEventsDb(dbPath);
			if (events.length >= expectedCount) {
				return events;
			}
		}
		// Busy-wait 200ms between polls
		const waitUntil = Date.now() + 200;
		while (Date.now() < waitUntil) {
			/* spin */
		}
	}
	// Last attempt
	if (existsSync(dbPath)) {
		return readEventsDb(dbPath);
	}
	return [];
}

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

	const scryrsDir = dirname(scryrsPath);
	const subprocessEnv = {
		...process.env,
		PATH: `${scryrsDir}:${process.env.PATH || ""}`,
		SCRYRS_DEBUG: "",
	};

	function runTsxScript(scriptFile, envOverrides = {}) {
		const npx = join(dir, "node_modules", ".bin", "tsx");
		const result = spawnSync(npx, [scriptFile], {
			env: { ...subprocessEnv, ...envOverrides },
			cwd: dir,
			timeout: 15000,
			encoding: "utf-8",
			stdio: ["ignore", "pipe", "pipe"],
		});

		return {
			ok: result.status === 0,
			stdout: result.stdout?.toString() || "",
			stderr: result.stderr?.toString() || "",
			status: result.status,
			signal: result.signal,
		};
	}

	return {
		/**
		 * Invoke the hook for a given event, wait for it to complete,
		 * and return the result.
		 */
		async runSessionStart(reason, envOverrides = {}) {
			const scriptFile = join(dir, "invoke-session.mjs");
			// We write a self-contained script that loads the hook, fires the event, and exits.
			writeFileSync(
				scriptFile,
				`
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);

// The pi hook imports '@earendil-works/pi-coding-agent' as a type-only import.
// tsx strips type-only imports, so this is safe.
// The fake exec spawns real scryrs using Pi's actual exec semantics:
// stdin ignored, stdout/stderr captured, timeout enforced.
const { spawnSync } = require("node:child_process");

const fakeApi = {
  handlers: {},
  on(event, handler) {
    this.handlers[event] = handler;
  },
  async exec(command, args, options) {
	const timeout = options?.timeout ?? 5000;
	const cwd = options?.cwd ?? process.cwd();
	const result = spawnSync(command, args, {
	  timeout,
	  encoding: "utf-8",
	  stdio: ["ignore", "pipe", "pipe"],
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
        setTimeout(() => process.exit(0), 2000);
      }).catch((err) => {
        console.error("HOOK_ERROR:", err.message);
        process.exit(1);
      });
    } else {
      // Handler returned void — give scryrs time then exit
      setTimeout(() => process.exit(0), 2000);
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

			return runTsxScript(scriptFile, envOverrides);
		},

		/**
		 * Invoke the hook for a tool_result event.
		 */
		async runToolResult(
			toolName,
			input,
			isError = false,
			options = {},
		) {
			const { envOverrides = {}, content = null, details = null } = options;
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
	const timeout = options?.timeout ?? 5000;
	const cwd = options?.cwd ?? process.cwd();
	const result = spawnSync(command, args, {
	  timeout,
	  encoding: "utf-8",
	  stdio: ["ignore", "pipe", "pipe"],
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
    content: ${JSON.stringify(content)},
    details: ${JSON.stringify(details)},
    isError: ${isError},
  };

  const preSnapshot = JSON.stringify(event);

  handler(event, {}).then((result) => {
    // Output: result|preSnapshot|postSnapshot
    console.log("RESULT:" + JSON.stringify(result));
    console.log("PRESNAPSHOT:" + preSnapshot);
    console.log("POSTSNAPSHOT:" + JSON.stringify(event));
    setTimeout(() => process.exit(0), 1000);
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

			return runTsxScript(scriptFile, envOverrides);
		},

		/**
		 * Run fail-open non-zero-exit test: invoke tool_result with a fake
		 * scryrs that resolves with a non-zero exit code.
		 */
		async runFailOpenNonZeroExit(toolName, input, envOverrides = {}) {
			const scriptFile = join(dir, "invoke-nonzero.mjs");
			writeFileSync(
				scriptFile,
				`
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { spawnSync } = require("node:child_process");

const errors = [];
const origError = console.error;
console.error = (...args) => { errors.push(args.join(" ")); };

const fakeApi = {
  handlers: {},
  on(event, handler) {
    this.handlers[event] = handler;
  },
  async exec(command, args, options) {
    // Simulate a resolved non-zero exit (e.g., scryrs rejected the line)
    return { stdout: "", stderr: "", code: 1, killed: false };
  },
};

import(${JSON.stringify(HOOK_SOURCE)}).then((mod) => {
  mod.default(fakeApi);

  const handler = fakeApi.handlers["tool_result"];
  if (!handler) { process.exit(0); }

  const event = {
    toolName: ${JSON.stringify(toolName)},
    toolCallId: "call-test-456",
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
    setTimeout(() => process.exit(0), 1000);
  }).catch((err) => {
    console.error("UNEXPECTED_CRASH:", err.message);
    process.exit(1);
  });
}).catch((err) => {
  console.error("IMPORT_ERROR:", err.message);
  process.exit(1);
});
`,
			);

			return runTsxScript(scriptFile, envOverrides);
		},

		/**
		 * Run fail-open test: invoke tool_result with scryrs not on PATH.
		 */
		async runFailOpen(toolName, input, envOverrides = {}) {
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
	const timeout = options?.timeout ?? 5000;
	const result = spawnSync(command, args, {
	  timeout,
	  encoding: "utf-8",
	  stdio: ["ignore", "pipe", "pipe"],
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
    setTimeout(() => process.exit(0), 1000);
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

			return runTsxScript(scriptFile, envOverrides);
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

function assertContains(text, needle, name) {
	if (text.includes(needle)) {
		pass(name);
	} else {
		fail(name, `missing ${JSON.stringify(needle)} in ${JSON.stringify(text)}`);
	}
}

function assertNotContains(text, needle, name) {
	if (!text.includes(needle)) {
		pass(name);
	} else {
		fail(name, `unexpected ${JSON.stringify(needle)} in ${JSON.stringify(text)}`);
	}
}

// -----------------------------------------------------------------------
// Test: Debug disabled stays quiet
// -----------------------------------------------------------------------
async function testDebugDisabledQuiet() {
	console.log(`\n\x1b[33m--- Pi: Debug Disabled ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = SCRYRS_BIN;
	const runner = createHookRunner(dir, scryrsPath);

	const sessionResult = await runner.runSessionStart("debug-disabled");
	const toolResult = await runner.runToolResult("bash", {
		command: "echo quiet",
	});

	if (!sessionResult.ok) {
		fail(
			"Pi debug disabled: session_start",
			`hook invocation failed: ${sessionResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	if (!toolResult.ok) {
		fail(
			"Pi debug disabled: tool_result",
			`hook invocation failed: ${toolResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	assertNotContains(
		sessionResult.stderr,
		"[scryrs]",
		"Pi debug disabled: no hook debug lines on session_start",
	);
	assertNotContains(
		toolResult.stderr,
		"[scryrs]",
		"Pi debug disabled: no hook debug lines on tool_result",
	);
	assertNotContains(
		`${sessionResult.stderr}\n${toolResult.stderr}`,
		"[scryrs-record]",
		"Pi debug disabled: no record debug lines echoed",
	);

	rmSync(dir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Debug breadcrumbs
// -----------------------------------------------------------------------
async function testDebugBreadcrumbs() {
	console.log(`\n\x1b[33m--- Pi: Debug Breadcrumbs ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = SCRYRS_BIN;
	const runner = createHookRunner(dir, scryrsPath);
	const debugEnv = { SCRYRS_DEBUG: "1" };

	const sessionResult = await runner.runSessionStart("debug-enabled", debugEnv);
	if (!sessionResult.ok) {
		fail(
			"Pi debug: session_start",
			`hook invocation failed: ${sessionResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	assertContains(
		sessionResult.stderr,
		"[scryrs] stage=hook_load",
		"Pi debug: hook load breadcrumb",
	);
	assertContains(
		sessionResult.stderr,
		"[scryrs] stage=session_start",
		"Pi debug: session_start breadcrumb",
	);
	assertContains(
		sessionResult.stderr,
		"[scryrs] stage=record_send trace_event=SessionStart",
		"Pi debug: session_start record send breadcrumb",
	);
	assertContains(
		sessionResult.stderr,
		"[scryrs] stage=record_result trace_event=SessionStart",
		"Pi debug: session_start record result breadcrumb",
	);
	assertContains(
		sessionResult.stderr,
		"[scryrs-record] stage=accepted",
		"Pi debug: record stderr echoed for session_start",
	);

	const trackedResult = await runner.runToolResult(
		"bash",
		{ command: "cargo test" },
		false,
		{ envOverrides: debugEnv },
	);
	if (!trackedResult.ok) {
		fail(
			"Pi debug: tracked tool",
			`hook invocation failed: ${trackedResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	assertContains(
		trackedResult.stderr,
		"[scryrs] stage=tool_result tool=bash tracked=false is_error=false input_keys=command",
		"Pi debug: bash (debug-gated) breadcrumb",
	);
	assertContains(
		trackedResult.stderr,
		"[scryrs] stage=trace_mapped tool=bash trace_event=CommandExecuted",
		"Pi debug: mapped trace breadcrumb",
	);
	assertContains(
		trackedResult.stderr,
		"[scryrs] stage=record_send trace_event=CommandExecuted tool=bash",
		"Pi debug: tracked record send breadcrumb",
	);
	assertContains(
		trackedResult.stderr,
		"[scryrs] stage=record_result trace_event=CommandExecuted tool=bash",
		"Pi debug: tracked record result breadcrumb",
	);

	const untrackedResult = await runner.runToolResult(
		"web_search",
		{ query: "debug search" },
		false,
		{ envOverrides: debugEnv },
	);
	if (!untrackedResult.ok) {
		fail(
			"Pi debug: untracked tool",
			`hook invocation failed: ${untrackedResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	assertContains(
		untrackedResult.stderr,
		"[scryrs] stage=tool_result tool=web_search tracked=false is_error=false input_keys=query",
		"Pi debug: untracked tool breadcrumb",
	);

	const missingFieldResult = await runner.runToolResult(
		"read",
		{ other: "value" },
		false,
		{ envOverrides: debugEnv },
	);
	if (!missingFieldResult.ok) {
		fail(
			"Pi debug: missing field",
			`hook invocation failed: ${missingFieldResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	assertContains(
		missingFieldResult.stderr,
		"[scryrs] stage=missing_field tool=read wanted_field=path available_keys=other fallback=unknown",
		"Pi debug: missing field breadcrumb",
	);

	const wireResult = await runner.runToolResult(
		"bash",
		{ command: "echo wire", secret: "hide-this" },
		false,
		{
			envOverrides: { SCRYRS_DEBUG: "wire" },
			content: "full content should stay out of default logs",
			details: { hidden: "details should stay bounded" },
		},
	);
	if (!wireResult.ok) {
		fail(
			"Pi debug: wire mode",
			`hook invocation failed: ${wireResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	assertContains(
		wireResult.stderr,
		"[scryrs] stage=tool_input_wire tool=bash preview=command:echo wire",
		"Pi debug: wire preview breadcrumb",
	);
	assertNotContains(
		wireResult.stderr,
		"full content should stay out of default logs",
		"Pi debug: wire mode omits full content",
	);

	const nonZeroResult = await runner.runFailOpenNonZeroExit(
		"read",
		{ path: "/debug/nonzero.txt" },
		debugEnv,
	);
	if (!nonZeroResult.ok) {
		fail(
			"Pi debug: non-zero record exit",
			`hook invocation failed: ${nonZeroResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	const nonZeroParsed = parseFixtureOutput(nonZeroResult.stdout);
	const nonZeroErrors = JSON.stringify(nonZeroParsed.errors || []);
	assertContains(
		nonZeroErrors,
		"[scryrs] stage=record_result",
		"Pi debug: non-zero exit result breadcrumb",
	);
	assertContains(
		nonZeroErrors,
		"[scryrs] stage=record_nonzero",
		"Pi debug: non-zero exit breadcrumb",
	);

	const execFailureResult = await runner.runFailOpen(
		"read",
		{ path: "/debug/missing.txt" },
		debugEnv,
	);
	if (!execFailureResult.ok) {
		fail(
			"Pi debug: exec failure",
			`hook invocation failed: ${execFailureResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	const execFailureParsed = parseFixtureOutput(execFailureResult.stdout);
	const execFailureErrors = JSON.stringify(execFailureParsed.errors || []);
	assertContains(
		execFailureErrors,
		"[scryrs] stage=record_exec_error",
		"Pi debug: exec failure breadcrumb",
	);

	const eventsDb = join(dir, ".scryrs", "scryrs.db");
	const events = waitForEventCount(eventsDb, 4, 10000);
	const untrackedEvents = events.filter((event) => event.tool_name === "web_search");
	if (untrackedEvents.length === 0) {
		pass("Pi debug: untracked tool still not persisted");
	} else {
		fail(
			"Pi debug: untracked tool persistence",
			`unexpected events: ${JSON.stringify(untrackedEvents)}`,
		);
	}

	rmSync(dir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Successful capture — SessionStart + five default tracked tools
// -----------------------------------------------------------------------
async function testSuccessfulCapture() {
	console.log(`\n\x1b[33m--- Pi: Successful Capture ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = SCRYRS_BIN;
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

	// 2. Emit tool_result for each of the five default tracked tools
	const tools = [
		{
			name: "read",
			input: { path: "/src/main.rs" },
			expectedType: "FileOpened",
			payloadKey: "path",
			payloadVal: "/src/main.rs",
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

	// 3. Assert scryrs.db (poll to avoid SessionStart fire-and-forget race)
	const eventsDb = join(dir, ".scryrs", "scryrs.db");
	const events = waitForEventCount(eventsDb, 6, 5000);

	// Should have SessionStart + 5 tool events = 6
	if (events.length < 6) {
		fail(
			"Pi events count",
			`expected at least 6 events (SessionStart + 5 tools), got ${events.length}`,
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
		fail("Pi SessionStart", "no SessionStart event found in scryrs.db");
	}

	// Check each tool event
	for (const tool of tools) {
		const toolEvent = events.find(
			(e) => e.event_type === tool.expectedType && e.tool_name === tool.name,
		);
		if (!toolEvent) {
			fail(
				`Pi ${tool.name}`,
				`no ${tool.expectedType} event for ${tool.name} found in scryrs.db`,
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
	const scryrsPath = SCRYRS_BIN;
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

	// Assert scryrs.db contains FailedLookup
	const eventsDb = join(dir, ".scryrs", "scryrs.db");
	const events = readEventsDb(eventsDb);
	const failedEvent = events.find((e) => e.event_type === "FailedLookup");

	if (!failedEvent) {
		fail("Pi failure: FailedLookup", "no FailedLookup event in scryrs.db");
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
// Test: Fail-open — scryrs resolve non-zero exit
// -----------------------------------------------------------------------
async function testFailOpenNonZeroExit() {
	console.log(`\n\x1b[33m--- Pi: Fail-open (non-zero exit) ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = SCRYRS_BIN;
	const runner = createHookRunner(dir, scryrsPath);

	const foResult = await runner.runFailOpenNonZeroExit("read", {
		path: "/test.txt",
	});

	if (!foResult.ok) {
		fail(
			"Pi fail-open nonzero",
			`subprocess crash: ${foResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	const parsed = parseFixtureOutput(foResult.stdout);

	// Handler should return undefined
	if (parsed.result !== undefined) {
		fail(
			"Pi fail-open nonzero: handler return",
			`expected undefined, got ${JSON.stringify(parsed.result)}`,
		);
	} else {
		pass("Pi fail-open nonzero: handler returned undefined (non-interference)");
	}

	// Original event should be unchanged
	if (parsed.preSnapshot && parsed.postSnapshot) {
		if (
			JSON.stringify(parsed.preSnapshot) === JSON.stringify(parsed.postSnapshot)
		) {
			pass("Pi fail-open nonzero: original payload unchanged");
		} else {
			fail("Pi fail-open nonzero: payload mutated");
		}
	}

	// console.error should have been called with a trace gap message
	if (parsed.errors && parsed.errors.length > 0) {
		const hasTraceGap = parsed.errors.some(
			(e) => e.includes("exited non-zero") || e.includes("trace gap"),
		);
		if (hasTraceGap) {
			pass("Pi fail-open nonzero: console.error called with trace gap message");
		} else {
			fail(
				"Pi fail-open nonzero: console.error",
				`no trace-gap message found in: ${JSON.stringify(parsed.errors)}`,
			);
		}
	} else {
		fail(
			"Pi fail-open nonzero: console.error",
			"no console.error output — non-zero exit may not be logged",
		);
	}

	rmSync(dir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Fail-open — scryrs missing
// -----------------------------------------------------------------------
async function testFailOpen() {
	console.log(`\n\x1b[33m--- Pi: Fail-open ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = SCRYRS_BIN;
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
	const eventsDb = join(dir, ".scryrs", "scryrs.db");
	if (existsSync(eventsDb)) {
		const events = readEventsDb(eventsDb);
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
	const scryrsPath = SCRYRS_BIN;
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
	const eventsDb = join(dir, ".scryrs", "scryrs.db");
	const events = readEventsDb(eventsDb);
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
// Test: Rewrite-tool compatibility — RTK-style Bash commands on tool_result
// -----------------------------------------------------------------------
async function testRewriteCompatibility() {
	console.log(`\n\x1b[33m--- Pi: Rewrite-tool Compatibility ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = SCRYRS_BIN;
	const runner = createHookRunner(dir, scryrsPath);

	// 1. Emit SessionStart
	const ssResult = await runner.runSessionStart("rtk-test");
	if (!ssResult.ok) {
		fail(
			"Pi RTK SessionStart",
			`hook invocation failed: ${ssResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}
	pass("Pi RTK: SessionStart invoked without error");

	// 2a. Simple RTK-prefixed Bash command via tool_result (debug-gated)
	const simpleResult = await runner.runToolResult(
		"bash",
		{ command: "rtk ls -la" },
		false,
		{ envOverrides: { SCRYRS_DEBUG: "1" } },
	);
	if (!simpleResult.ok) {
		fail(
			"Pi RTK simple",
			`hook invocation failed: ${simpleResult.stderr?.slice(0, 200)}`,
		);
	} else {
		pass("Pi RTK simple: hook invoked without error");

		const parsed = parseFixtureOutput(simpleResult.stdout);

		// Assert handler returns undefined (non-interference)
		if (parsed.result !== undefined) {
			fail(
				"Pi RTK simple: handler return",
				`expected undefined, got ${JSON.stringify(parsed.result)}`,
			);
		} else {
			pass("Pi RTK simple: handler returned undefined (non-interference)");
		}

		// Assert original event input unchanged (deep equality)
		if (parsed.preSnapshot && parsed.postSnapshot) {
			if (
				JSON.stringify(parsed.preSnapshot) ===
				JSON.stringify(parsed.postSnapshot)
			) {
				pass("Pi RTK simple: original event input unchanged");
			} else {
				fail("Pi RTK simple: event input mutated");
			}
		} else {
			fail("Pi RTK simple: snapshot", "missing pre/post snapshot");
		}
	}

	// 2b. Compound rewritten Bash command via tool_result (debug-gated)
	const compoundCmd =
		'echo "=== BACKEND ===" && rtk ls backend/api/ && rtk ls backend/cmd/';
	const compoundResult = await runner.runToolResult(
		"bash",
		{ command: compoundCmd },
		false,
		{ envOverrides: { SCRYRS_DEBUG: "1" } },
	);
	if (!compoundResult.ok) {
		fail(
			"Pi RTK compound",
			`hook invocation failed: ${compoundResult.stderr?.slice(0, 200)}`,
		);
	} else {
		pass("Pi RTK compound: hook invoked without error");

		const parsed = parseFixtureOutput(compoundResult.stdout);

		if (parsed.result !== undefined) {
			fail(
				"Pi RTK compound: handler return",
				`expected undefined, got ${JSON.stringify(parsed.result)}`,
			);
		} else {
			pass("Pi RTK compound: handler returned undefined (non-interference)");
		}

		if (parsed.preSnapshot && parsed.postSnapshot) {
			if (
				JSON.stringify(parsed.preSnapshot) ===
				JSON.stringify(parsed.postSnapshot)
			) {
				pass("Pi RTK compound: original event input unchanged");
			} else {
				fail("Pi RTK compound: event input mutated");
			}
		} else {
			fail("Pi RTK compound: snapshot", "missing pre/post snapshot");
		}
	}

	const eventsDb = join(dir, ".scryrs", "scryrs.db");

	// 3a. After simple command, assert it persisted as observed.
	const afterSimple = waitForEventCount(eventsDb, 2, 15000);
	const simplePersisted = afterSimple.find(
		(e) =>
			e.event_type === "CommandExecuted" &&
			e.tool_name === "bash" &&
			e.payload?.command === "rtk ls -la",
	);
	if (simplePersisted) {
		pass("Pi RTK: simple command persisted as 'rtk ls -la'");
		const shapeOk = assertEventShape(simplePersisted, "CommandExecuted", "bash");
		if (shapeOk) {
			pass("Pi RTK simple: envelope shape correct");
		}
	} else {
		fail(
			"Pi RTK: simple command",
			`expected payload.command='rtk ls -la', got: ${JSON.stringify(afterSimple.map((e) => e.payload?.command))}`,
		);
	}

	// 3b. Assert final scryrs.db contents (poll to avoid SessionStart fire-and-forget race)
	const events = waitForEventCount(eventsDb, 3, 15000);

	const bashEvents = events.filter(
		(e) => e.event_type === "CommandExecuted" && e.tool_name === "bash",
	);

	if (bashEvents.length !== 2) {
		fail(
			"Pi RTK: bash events count",
			`expected 2 Bash events, got ${bashEvents.length}`,
		);
	} else {
		pass("Pi RTK: 2 Bash events persisted");
	}

	// Check compound rewritten command persisted exactly
	const rtkCompound = bashEvents.find(
		(e) => e.payload?.command === compoundCmd,
	);
	if (rtkCompound) {
		pass("Pi RTK: compound command persisted exactly as observed");
		const shapeOk = assertEventShape(rtkCompound, "CommandExecuted", "bash");
		if (shapeOk) {
			pass("Pi RTK compound: envelope shape correct");
		}
	} else {
		fail(
			"Pi RTK: compound command",
			`expected payload.command='${compoundCmd}', got: ${JSON.stringify(bashEvents.map((e) => e.payload?.command))}`,
		);
	}

	rmSync(dir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Default mode excludes Bash
// -----------------------------------------------------------------------
async function testDefaultModeExcludesBash() {
	console.log(`\n\x1b[33m--- Pi: Default Mode Excludes Bash ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = SCRYRS_BIN;
	const runner = createHookRunner(dir, scryrsPath);

	// No SCRYRS_DEBUG — bash should not be captured
	const trResult = await runner.runToolResult(
		"bash",
		{ command: "cargo build" },
		false,
	);

	if (!trResult.ok) {
		fail(
			"Pi default-no-bash",
			`hook invocation failed: ${trResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	const parsed = parseFixtureOutput(trResult.stdout);

	// Handler should return undefined (non-interference)
	if (parsed.result !== undefined) {
		fail(
			"Pi default-no-bash: handler return",
			`expected undefined, got ${JSON.stringify(parsed.result)}`,
		);
	} else {
		pass("Pi default-no-bash: handler returned undefined (non-interference)");
	}

	// No CommandExecuted event should be persisted
	const eventsDb = join(dir, ".scryrs", "scryrs.db");
	const events = readEventsDb(eventsDb);
	const bashEvents = events.filter(
		(e) => e.event_type === "CommandExecuted",
	);

	if (bashEvents.length === 0) {
		pass("Pi default-no-bash: no CommandExecuted event persisted");
	} else {
		fail(
			"Pi default-no-bash",
			`expected 0 Bash events, got ${bashEvents.length}`,
		);
	}

	rmSync(dir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Debug mode captures Bash as CommandExecuted
// -----------------------------------------------------------------------
async function testDebugModeCapturesBash() {
	console.log(`\n\x1b[33m--- Pi: Debug Mode Captures Bash ---\x1b[0m`);

	const dir = tempDir();
	const scryrsPath = SCRYRS_BIN;
	const runner = createHookRunner(dir, scryrsPath);

	// Emit Bash with SCRYRS_DEBUG=1
	const trResult = await runner.runToolResult(
		"bash",
		{ command: "rtk ls -la" },
		false,
		{ envOverrides: { SCRYRS_DEBUG: "1" } },
	);

	if (!trResult.ok) {
		fail(
			"Pi debug-bash",
			`hook invocation failed: ${trResult.stderr?.slice(0, 200)}`,
		);
		rmSync(dir, { recursive: true, force: true });
		return;
	}

	const parsed = parseFixtureOutput(trResult.stdout);

	if (parsed.result !== undefined) {
		fail(
			"Pi debug-bash: handler return",
			`expected undefined, got ${JSON.stringify(parsed.result)}`,
		);
	} else {
		pass("Pi debug-bash: handler returned undefined (non-interference)");
	}

	// Assert CommandExecuted event persisted
	const eventsDb = join(dir, ".scryrs", "scryrs.db");
	const events = readEventsDb(eventsDb);
	const bashEvents = events.filter(
		(e) => e.event_type === "CommandExecuted" && e.tool_name === "bash",
	);

	if (bashEvents.length !== 1) {
		fail(
			"Pi debug-bash: count",
			`expected 1 Bash event, got ${bashEvents.length}`,
		);
	} else {
		pass("Pi debug-bash: 1 CommandExecuted event persisted");
	}

	const rtkEvent = bashEvents.find(
		(e) => e.payload?.command === "rtk ls -la",
	);
	if (rtkEvent) {
		pass("Pi debug-bash: payload.command is 'rtk ls -la'");
		const shapeOk = assertEventShape(rtkEvent, "CommandExecuted", "bash");
		if (shapeOk) {
			pass("Pi debug-bash: envelope shape correct");
		}
	} else {
		fail(
			"Pi debug-bash: payload",
			`expected 'rtk ls -la', got: ${JSON.stringify(bashEvents.map((e) => e.payload?.command))}`,
		);
	}

	rmSync(dir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Main
// -----------------------------------------------------------------------
async function main() {
	await testDebugDisabledQuiet();
	await testDebugBreadcrumbs();
	await testSuccessfulCapture();
	await testDefaultModeExcludesBash();
	await testDebugModeCapturesBash();
	await testRewriteCompatibility();
	await testFailurePropagation();
	await testFailOpenNonZeroExit();
	await testFailOpen();
	await testUnlistedTools();

	summary();
}

main().catch((err) => {
	console.error("Pi E2E fixture crashed:", err);
	process.exit(2);
});
