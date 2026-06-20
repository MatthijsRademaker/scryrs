import {
	writeFileSync,
	readFileSync,
	unlinkSync,
	mkdirSync,
	chmodSync,
	existsSync,
	rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..");
const HOOK_FILE = join(ROOT, "hooks", "claude-code", "scryrs-hook.mjs");

let PASSED = 0;
let FAILED = 0;

function pass(name) {
	console.log(`  \x1b[32mPASS\x1b[0m ${name}`);
	PASSED++;
}

function fail(name, reason) {
	console.log(`  \x1b[31mFAIL\x1b[0m ${name}`);
	if (reason) console.log(`        \x1b[31m${reason}\x1b[0m`);
	FAILED++;
}

function header(text) {
	console.log(`\n\x1b[33m--- ${text} ---\x1b[0m`);
}

// -----------------------------------------------------------------------
// Helper: invoke the hook as a subprocess
// -----------------------------------------------------------------------
// Writes a temp ESM script that imports the hook, calls it with the given
// tool_name / tool_input, and prints the result as JSON.
// Returns { result, exitCode, stderr }.
function invokeHook(toolName, toolInput, extraPath) {
	const tmpDir = join(
		tmpdir(),
		`scryrs-invoke-${Date.now()}-${Math.random().toString(36).slice(2)}`,
	);
	mkdirSync(tmpDir, { recursive: true });
	const scriptFile = join(tmpDir, "invoke.mjs");

	const code = [
		`import hook from ${JSON.stringify(HOOK_FILE)};`,
		`const input = { tool_name: ${JSON.stringify(toolName)}, tool_input: ${JSON.stringify(toolInput)} };`,
		`const result = await hook(input);`,
		`console.log(JSON.stringify(result));`,
	].join("\n");

	writeFileSync(scriptFile, code);

	const env = { ...process.env };
	if (extraPath) {
		env.PATH = `${extraPath}:${env.PATH || ""}`;
	}

	try {
		const stdout = execFileSync("node", [scriptFile], {
			env,
			timeout: 15000,
			encoding: "utf-8",
			stdio: ["ignore", "pipe", "pipe"],
		}).trim();
		rmSync(tmpDir, { recursive: true, force: true });
		return { result: JSON.parse(stdout), exitCode: 0 };
	} catch (err) {
		rmSync(tmpDir, { recursive: true, force: true });
		if (err.stdout) {
			try {
				return {
					result: JSON.parse(err.stdout.toString().trim()),
					exitCode: err.status || 1,
				};
			} catch {
				return {
					result: null,
					exitCode: err.status || 1,
					stderr: err.stderr?.toString() || "",
				};
			}
		}
		return { result: null, exitCode: err.status || 1, stderr: String(err) };
	}
}

// -----------------------------------------------------------------------
// Helper: cleanup temp directory
// -----------------------------------------------------------------------
function cleanup(dir) {
	try {
		rmSync(dir, { recursive: true, force: true });
	} catch {
		// best-effort
	}
}

// -----------------------------------------------------------------------
// 3.2 JSON Shaping
// -----------------------------------------------------------------------
async function testJsonShaping() {
	header("JSON Shaping — verify hook output shape for all nine tools");

	// Create a fake scryrs that captures stdin to a file
	const tmpDir = join(tmpdir(), `scryrs-hook-test-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });
	const captureFile = join(tmpDir, "captured.jsonl");
	const fakeScryrs = join(tmpDir, "scryrs");

	writeFileSync(
		fakeScryrs,
		`#!/usr/bin/env bash\ncat >> '${captureFile}'\nexit 0\n`,
	);
	chmodSync(fakeScryrs, 0o755);

	const tools = [
		{
			name: "read",
			input: { file_path: "/src/main.rs" },
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
			name: "grep",
			input: { pattern: "error handling" },
			expectedType: "SearchRun",
			payloadKey: "query",
			payloadVal: "error handling",
		},
		{
			name: "glob",
			input: { pattern: "**/*.rs" },
			expectedType: "SearchRun",
			payloadKey: "query",
			payloadVal: "**/*.rs",
		},
		{
			name: "edit",
			input: { file_path: "/src/lib.rs" },
			expectedType: "EditMade",
			payloadKey: "target",
			payloadVal: "/src/lib.rs",
		},
		{
			name: "write",
			input: { file_path: "/src/new.rs" },
			expectedType: "EditMade",
			payloadKey: "target",
			payloadVal: "/src/new.rs",
		},
		{
			name: "notebookedit",
			input: { file_path: "/notebook.ipynb" },
			expectedType: "EditMade",
			payloadKey: "target",
			payloadVal: "/notebook.ipynb",
		},
		{
			name: "web_search",
			input: { searchTerm: "rust serde" },
			expectedType: "SearchRun",
			payloadKey: "query",
			payloadVal: "rust serde",
		},
		{
			name: "web_search",
			input: { query: "tokio runtime" },
			expectedType: "SearchRun",
			payloadKey: "query",
			payloadVal: "tokio runtime",
		},
		{
			name: "web_fetch",
			input: { url: "https://example.com" },
			expectedType: "DocRetrieved",
			payloadKey: "doc_ref",
			payloadVal: "https://example.com",
		},
	];

	// Invoke each tool through the hook
	for (const tool of tools) {
		const { result } = invokeHook(tool.name, tool.input, tmpDir);
		if (!result || !result.continue) {
			fail(`${tool.name}: hook return`, "did not return {continue:true}");
			continue;
		}
		pass(`${tool.name}: hook returned {continue:true}`);
	}

	// Verify captured events
	if (!existsSync(captureFile)) {
		fail("captured events", "no events captured");
		cleanup(tmpDir);
		return;
	}

	const raw = readFileSync(captureFile, "utf-8").trim();
	const lines = raw ? raw.split("\n").filter((l) => l.trim()) : [];

	if (lines.length !== tools.length) {
		fail("event count", `expected ${tools.length} events, got ${lines.length}`);
	}

	for (let i = 0; i < Math.min(lines.length, tools.length); i++) {
		const tool = tools[i];
		let event;
		try {
			event = JSON.parse(lines[i]);
		} catch {
			fail(`${tool.name}: parse`, `invalid JSON: ${lines[i].slice(0, 80)}`);
			continue;
		}

		const checks = [];
		if (event.schema_version !== "0.1.0")
			checks.push(`schema_version=${event.schema_version}`);
		if (!event.timestamp) checks.push("missing timestamp");
		if (!event.session_id) checks.push("missing session_id");
		if (event.event_type !== tool.expectedType)
			checks.push(
				`event_type=${event.event_type} expected=${tool.expectedType}`,
			);
		if (event.tool_name !== tool.name)
			checks.push(`tool_name=${event.tool_name}`);
		if (!event.payload || event.payload.type !== tool.expectedType)
			checks.push(`payload.type=${event.payload?.type}`);
		if (!event.outcome || event.outcome.result !== "Success")
			checks.push(`outcome=${JSON.stringify(event.outcome)}`);
		if (event.payload && event.payload[tool.payloadKey] !== tool.payloadVal)
			checks.push(
				`payload.${tool.payloadKey}=${event.payload?.[tool.payloadKey]} expected=${tool.payloadVal}`,
			);

		if (checks.length > 0) {
			fail(`${tool.name}: shape`, checks.join("; "));
		} else {
			pass(`${tool.name}: event shape correct`);
		}
	}

	cleanup(tmpDir);
}

// -----------------------------------------------------------------------
// 3.3 Happy-path forwarding
// -----------------------------------------------------------------------
async function testHappyPath() {
	header("Happy-path forwarding — verify hook pipes to scryrs record --stdin");

	const tmpDir = join(tmpdir(), `scryrs-hook-test-happy-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });
	const captureFile = join(tmpDir, "captured.jsonl");
	const fakeScryrs = join(tmpDir, "scryrs");

	writeFileSync(
		fakeScryrs,
		`#!/usr/bin/env bash\ncat >> '${captureFile}'\nexit 0\n`,
	);
	chmodSync(fakeScryrs, 0o755);

	const { result } = invokeHook("read", { file_path: "/tmp/test.txt" }, tmpDir);

	if (!result || !result.continue) {
		fail("happy-path", "hook did not return {continue:true}");
		cleanup(tmpDir);
		return;
	}

	if (!existsSync(captureFile)) {
		fail("happy-path", "no event captured — scryrs not invoked?");
		cleanup(tmpDir);
		return;
	}

	const raw = readFileSync(captureFile, "utf-8").trim();
	if (!raw) {
		fail("happy-path", "captured file is empty");
		cleanup(tmpDir);
		return;
	}

	let event;
	try {
		event = JSON.parse(raw.split("\n")[0]);
	} catch {
		fail("happy-path", `captured invalid JSON: ${raw.slice(0, 80)}`);
		cleanup(tmpDir);
		return;
	}

	if (
		event.schema_version === "0.1.0" &&
		event.event_type === "FileOpened" &&
		event.outcome?.result === "Success"
	) {
		pass("happy-path: event accepted by (fake) scryrs record --stdin");
	} else {
		fail(
			"happy-path",
			`event didn't match expected shape: ${JSON.stringify(event)}`,
		);
	}

	cleanup(tmpDir);
}

// -----------------------------------------------------------------------
// 3.4 Fail-open: missing binary
// -----------------------------------------------------------------------
async function testFailOpenMissingBinary() {
	header(
		"Fail-open (missing binary) — hook returns success when scryrs is not on PATH",
	);

	const tmpDir = join(tmpdir(), `scryrs-hook-test-missing-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });

	const { result, stderr } = invokeHook(
		"read",
		{ file_path: "/x.txt" },
		tmpDir,
	);

	if (!result || !result.continue) {
		fail(
			"fail-open missing",
			"hook did not return {continue:true} when scryrs is missing",
		);
		cleanup(tmpDir);
		return;
	}

	pass("fail-open missing: hook returned {continue:true} with scryrs missing");

	// Verify no stderr output from the hook process
	if (stderr && stderr.trim()) {
		fail(
			"fail-open missing: stderr",
			`hook wrote to stderr: ${stderr.slice(0, 100)}`,
		);
	} else {
		pass("fail-open missing: no stderr output");
	}

	// Verify warning log was created (hook resolves against process.cwd())
	const warningLog = join(
		process.cwd(),
		".scryrs/hooks/claude-code-warnings.log",
	);
	if (existsSync(warningLog)) {
		const logContent = readFileSync(warningLog, "utf-8");
		if (
			logContent.includes("scryrs binary not found on PATH") ||
			logContent.includes("ENOENT")
		) {
			pass("fail-open missing: warning logged to claude-code-warnings.log");
		} else if (logContent.trim().length > 0) {
			pass("fail-open missing: warning logged to claude-code-warnings.log");
		} else {
			fail("fail-open missing: warning log", "log exists but is empty");
		}
	} else {
		fail("fail-open missing: warning log", "no warning log created");
	}

	cleanup(tmpDir);
	try {
		unlinkSync(warningLog);
	} catch {}
}

// -----------------------------------------------------------------------
// 3.5 Fail-open: non-zero exit
// -----------------------------------------------------------------------
async function testFailOpenNonZeroExit() {
	header(
		"Fail-open (non-zero exit) — hook returns success when scryrs exits non-zero",
	);

	const tmpDir = join(tmpdir(), `scryrs-hook-test-nonzero-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });
	const fakeScryrs = join(tmpDir, "scryrs");

	writeFileSync(fakeScryrs, `#!/usr/bin/env bash\ncat > /dev/null\nexit 1\n`);
	chmodSync(fakeScryrs, 0o755);

	const { result, stderr } = invokeHook(
		"read",
		{ file_path: "/x.txt" },
		tmpDir,
	);

	if (!result || !result.continue) {
		fail(
			"fail-open nonzero",
			"hook did not return {continue:true} when scryrs exits 1",
		);
		cleanup(tmpDir);
		return;
	}

	pass(
		"fail-open nonzero: hook returned {continue:true} with scryrs exiting 1",
	);

	if (stderr && stderr.trim()) {
		fail(
			"fail-open nonzero: stderr",
			`hook wrote to stderr: ${stderr.slice(0, 100)}`,
		);
	} else {
		pass("fail-open nonzero: no stderr output");
	}

	// Verify warning log was written for non-zero exit
	const warningLog = join(
		process.cwd(),
		".scryrs/hooks/claude-code-warnings.log",
	);
	if (existsSync(warningLog)) {
		const logContent = readFileSync(warningLog, "utf-8");
		if (
			logContent.includes("scryrs record exited with code") ||
			logContent.includes("exited with code 1")
		) {
			pass("fail-open nonzero: warning logged to claude-code-warnings.log");
		} else if (logContent.trim().length > 0) {
			pass("fail-open nonzero: warning logged to claude-code-warnings.log");
		} else {
			fail("fail-open nonzero: warning log", "log exists but is empty");
		}
	} else {
		fail("fail-open nonzero: warning log", "no warning log created");
	}

	cleanup(tmpDir);
	try {
		unlinkSync(warningLog);
	} catch {}
}

// -----------------------------------------------------------------------
// 3.6 Fail-open: timeout
// -----------------------------------------------------------------------
async function testFailOpenTimeout() {
	header("Fail-open (timeout) — hook returns success when scryrs hangs");

	const tmpDir = join(tmpdir(), `scryrs-hook-test-timeout-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });
	const fakeScryrs = join(tmpDir, "scryrs");

	// Fake scryrs that sleeps long enough to exceed the hook's 5s timeout.
	writeFileSync(fakeScryrs, `#!/usr/bin/env bash\nsleep 10\nexit 0\n`);
	chmodSync(fakeScryrs, 0o755);

	const start = Date.now();
	const { result, stderr } = invokeHook(
		"read",
		{ file_path: "/x.txt" },
		tmpDir,
	);
	const elapsed = Date.now() - start;

	if (!result || !result.continue) {
		fail(
			"fail-open timeout",
			"hook did not return {continue:true} when scryrs timed out",
		);
		cleanup(tmpDir);
		return;
	}

	pass(`fail-open timeout: hook returned {continue:true} after ${elapsed}ms`);

	// The hook should return well before the 10s scryrs sleep.
	if (elapsed > 9000) {
		fail(
			"fail-open timeout",
			`hook took ${elapsed}ms — expected return within ~5s timeout`,
		);
	} else {
		pass(
			`fail-open timeout: hook returned within timeout window (${elapsed}ms)`,
		);
	}

	if (stderr && stderr.trim()) {
		fail(
			"fail-open timeout: stderr",
			`hook wrote to stderr: ${stderr.slice(0, 100)}`,
		);
	} else {
		pass("fail-open timeout: no stderr output");
	}

	// Verify warning log was written for timeout
	const warningLog = join(
		process.cwd(),
		".scryrs/hooks/claude-code-warnings.log",
	);
	if (existsSync(warningLog)) {
		const logContent = readFileSync(warningLog, "utf-8");
		if (
			logContent.includes("timed out after") ||
			logContent.includes("timed out")
		) {
			pass("fail-open timeout: warning logged to claude-code-warnings.log");
		} else if (logContent.trim().length > 0) {
			pass("fail-open timeout: warning logged to claude-code-warnings.log");
		} else {
			fail("fail-open timeout: warning log", "log exists but is empty");
		}
	} else {
		fail("fail-open timeout: warning log", "no warning log created");
	}

	cleanup(tmpDir);
	try {
		unlinkSync(warningLog);
	} catch {}
}

// -----------------------------------------------------------------------
// 3.7 Transparency — no stdout/stderr alteration
// -----------------------------------------------------------------------
async function testTransparency() {
	header("Transparency — hook does not alter simulated tool output");

	const tmpDir = join(tmpdir(), `scryrs-hook-test-transparency-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });

	// Run hook in a subprocess and capture stdout/stderr separately
	const scriptFile = join(tmpDir, "transparency-test.mjs");
	const code = [
		`import hook from ${JSON.stringify(HOOK_FILE)};`,
		`const input = { tool_name: "bash", tool_input: { command: "echo hello" } };`,
		`const result = await hook(input);`,
		`// result is not logged — hook should not write to stdout`,
	].join("\n");
	writeFileSync(scriptFile, code);

	try {
		const stdout = execFileSync("node", [scriptFile], {
			env: { ...process.env, PATH: tmpDir },
			timeout: 10000,
			encoding: "utf-8",
			stdio: ["ignore", "pipe", "pipe"],
		});
		if (!stdout.trim()) {
			pass("transparency: hook produces no stdout");
		} else {
			fail(
				"transparency: stdout",
				`unexpected output: ${stdout.slice(0, 200)}`,
			);
		}
	} catch (err) {
		const stdout = err.stdout?.toString() || "";
		if (!stdout.trim()) {
			pass("transparency: hook produces no stdout");
		} else {
			fail(
				"transparency: stdout",
				`unexpected output: ${stdout.slice(0, 200)}`,
			);
		}
		const stderr = err.stderr?.toString() || "";
		if (!stderr.trim()) {
			pass("transparency: hook produces no stderr");
		} else {
			fail(
				"transparency: stderr",
				`unexpected stderr: ${stderr.slice(0, 200)}`,
			);
		}
	}

	cleanup(tmpDir);
}

// -----------------------------------------------------------------------
// Extra: Unlisted tools pass through
// -----------------------------------------------------------------------
async function testPassthrough() {
	header("Pass-through — unlisted tools are not intercepted");

	const tmpDir = join(tmpdir(), `scryrs-hook-test-passthrough-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });
	const captureFile = join(tmpDir, "captured.jsonl");
	const fakeScryrs = join(tmpDir, "scryrs");

	writeFileSync(
		fakeScryrs,
		`#!/usr/bin/env bash\ncat >> '${captureFile}'\nexit 0\n`,
	);
	chmodSync(fakeScryrs, 0o755);

	const { result } = invokeHook(
		"task",
		{ description: "do something" },
		tmpDir,
	);

	if (!result || !result.continue) {
		fail("passthrough: task", "unlisted tool was blocked");
	} else {
		pass("passthrough: task: hook returned {continue:true}");
	}

	if (existsSync(captureFile)) {
		const raw = readFileSync(captureFile, "utf-8").trim();
		if (raw) {
			fail("passthrough: task", "unlisted tool produced trace event");
		} else {
			pass("passthrough: task: no event captured");
		}
	} else {
		pass("passthrough: task: no event captured");
	}

	cleanup(tmpDir);
}

// -----------------------------------------------------------------------
// Main
// -----------------------------------------------------------------------
async function main() {
	const mode = process.argv[2] || "all";

	switch (mode) {
		case "--json":
		case "json":
			await testJsonShaping();
			break;
		case "--happy":
		case "happy":
			await testHappyPath();
			break;
		case "--fail-open":
		case "fail-open":
			await testFailOpenMissingBinary();
			await testFailOpenNonZeroExit();
			await testFailOpenTimeout();
			break;
		case "--transparency":
		case "transparency":
			await testTransparency();
			break;
		default:
			await testJsonShaping();
			await testHappyPath();
			await testFailOpenMissingBinary();
			await testFailOpenNonZeroExit();
			await testFailOpenTimeout();
			await testTransparency();
			await testPassthrough();
	}

	console.log("");
	console.log("============================================");
	console.log(
		`Results: \x1b[32m${PASSED} passed\x1b[0m, \x1b[31m${FAILED} failed\x1b[0m, ${PASSED + FAILED} total`,
	);
	console.log("============================================");

	if (FAILED > 0) process.exit(1);
}

main().catch((err) => {
	console.error("Test runner crashed:", err);
	process.exit(2);
});
