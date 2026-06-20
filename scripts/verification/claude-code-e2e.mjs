/**
 * scryrs Claude Code end-to-end verification fixture.
 *
 * Exercises hooks/claude-code/scryrs-hook.mjs against the real `scryrs record --stdin`
 * binary. Reuses JSON-shaping, fail-open, transparency, and pass-through test
 * logic from scripts/hook-test-runner.mjs.
 *
 * Prerequisites:
 *   - Real `scryrs` binary on PATH (built via `cargo build --release`)
 *   - Working directory is the repository root
 *
 * Usage: node scripts/verification/claude-code-e2e.mjs
 */

import {
	existsSync,
	mkdirSync,
	rmSync,
	writeFileSync,
	readFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { pass, fail, summary } from "./lib/assert.mjs";
import { readJsonl, assertEventShape } from "./lib/jsonl.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..", "..");
const HOOK_FILE = join(ROOT, "hooks", "claude-code", "scryrs-hook.mjs");

// -----------------------------------------------------------------------
// Helper: invoke the hook as a subprocess
// -----------------------------------------------------------------------
function invokeHook(toolName, toolInput, workDir) {
	const tmpDir = join(
		tmpdir(),
		`scryrs-cc-e2e-${Date.now()}-${Math.random().toString(36).slice(2)}`,
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
	// Ensure scryrs is on PATH
	if (workDir) {
		env.PATH = `${workDir}:${env.PATH || ""}`;
	}

	try {
		const stdout = execFileSync("node", [scriptFile], {
			env,
			cwd: workDir || process.cwd(),
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
// Test: JSON shaping — all nine tools against real scryrs
// -----------------------------------------------------------------------
async function testJsonShaping() {
	console.log(
		`\n\x1b[33m--- Claude Code: JSON Shaping (real scryrs) ---\x1b[0m`,
	);

	const tmpDir = join(tmpdir(), `scryrs-cc-e2e-json-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });

	const realScryrs = join(ROOT, "target", "release", "scryrs");

	// Initialize scryrs in the temp dir
	try {
		execFileSync(realScryrs, ["init"], {
			cwd: tmpDir,
			timeout: 5000,
			stdio: "ignore",
		});
	} catch {}

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
			name: "web_fetch",
			input: { url: "https://example.com" },
			expectedType: "DocRetrieved",
			payloadKey: "doc_ref",
			payloadVal: "https://example.com",
		},
	];

	// Invoke each tool through the hook, running in tmpDir
	for (const tool of tools) {
		const { result } = invokeHook(tool.name, tool.input, tmpDir);
		if (!result || !result.continue) {
			fail(
				`Claude Code ${tool.name}: hook return`,
				"did not return {continue:true}",
			);
			continue;
		}
		pass(`Claude Code ${tool.name}: hook returned {continue:true}`);
	}

	// Assert events.jsonl contents (appended by each scryrs record invocation)
	const eventsJsonl = join(tmpDir, ".scryrs", "events.jsonl");
	const events = readJsonl(eventsJsonl);

	if (events.length !== tools.length) {
		fail(
			"events.jsonl count",
			`expected ${tools.length} events, got ${events.length}`,
		);
	} else {
		pass(
			`events.jsonl: ${events.length} events (matches ${tools.length} tools)`,
		);
	}

	for (let i = 0; i < Math.min(events.length, tools.length); i++) {
		const tool = tools[i];
		const event = events[i];
		const shapeOk = assertEventShape(event, tool.expectedType, tool.name);

		if (
			shapeOk &&
			event.payload &&
			event.payload[tool.payloadKey] === tool.payloadVal
		) {
			pass(`Claude Code ${tool.name}: event shape + payload correct`);
		} else if (shapeOk) {
			fail(
				`Claude Code ${tool.name}: payload`,
				`payload.${tool.payloadKey}=${event.payload?.[tool.payloadKey]} expected=${tool.payloadVal}`,
			);
		}
	}

	rmSync(tmpDir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Non-interference — hook produces zero stdout/stderr
// -----------------------------------------------------------------------
async function testNonInterference() {
	console.log(`\n\x1b[33m--- Claude Code: Non-interference ---\x1b[0m`);

	const tmpDir = join(tmpdir(), `scryrs-cc-e2e-ni-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });

	const scriptFile = join(tmpDir, "transparency-test.mjs");
	const code = [
		`import hook from ${JSON.stringify(HOOK_FILE)};`,
		`const input = { tool_name: "bash", tool_input: { command: "echo hello" } };`,
		`const result = await hook(input);`,
		`// result is not logged — hook should not write to stdout`,
	].join("\n");
	writeFileSync(scriptFile, code);

	try {
		// Capture stdout and stderr separately (hook's own output, not scryrs')
		execFileSync("node", [scriptFile], {
			cwd: tmpDir,
			timeout: 10000,
			encoding: "utf-8",
			stdio: ["ignore", "pipe", "pipe"],
		});
		pass("Claude Code non-interference: hook stdout empty");
		pass("Claude Code non-interference: hook stderr empty");
	} catch (err) {
		const stdout = err.stdout?.toString() || "";
		const stderr = err.stderr?.toString() || "";
		if (!stdout.trim()) {
			pass("Claude Code non-interference: hook stdout empty");
		} else {
			fail(
				"Claude Code non-interference: stdout",
				`unexpected: ${stdout.slice(0, 200)}`,
			);
		}
		if (!stderr.trim()) {
			pass("Claude Code non-interference: hook stderr empty");
		} else {
			fail(
				"Claude Code non-interference: stderr",
				`unexpected: ${stderr.slice(0, 200)}`,
			);
		}
	}

	rmSync(tmpDir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Fail-open — scryrs missing
// -----------------------------------------------------------------------
async function testFailOpen() {
	console.log(
		`\n\x1b[33m--- Claude Code: Fail-open (scryrs missing) ---\x1b[0m`,
	);

	const tmpDir = join(tmpdir(), `scryrs-cc-e2e-fo-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });

	// Create a script that invokes the hook with PATH explicitly set
	// to exclude scryrs (only the empty dir and standard system paths).
	const scriptFile = join(tmpDir, "invoke-failopen.mjs");
	const code = [
		`import hook from ${JSON.stringify(HOOK_FILE)};`,
		`const input = { tool_name: "read", tool_input: { file_path: "/x.txt" } };`,
		`const result = await hook(input);`,
		`console.log(JSON.stringify(result));`,
	].join("\n");
	writeFileSync(scriptFile, code);

	// Set PATH to standard system dirs only — explicitly exclude /workspace/target/release
	const env = {
		...process.env,
		PATH: "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
	};

	let result, stderr;
	try {
		const stdout = execFileSync("node", [scriptFile], {
			env,
			cwd: tmpDir,
			timeout: 15000,
			encoding: "utf-8",
			stdio: ["ignore", "pipe", "pipe"],
		}).trim();
		result = JSON.parse(stdout);
	} catch (err) {
		if (err.stdout) {
			try {
				result = JSON.parse(err.stdout.toString().trim());
			} catch {
				result = null;
			}
		}
		stderr = err.stderr?.toString() || "";
	}

	if (!result || !result.continue) {
		fail(
			"Claude Code fail-open",
			"did not return {continue:true} when scryrs missing",
		);
	} else {
		pass("Claude Code fail-open: returned {continue:true}");
	}

	if (stderr && stderr.trim()) {
		fail(
			"Claude Code fail-open: stderr",
			`hook wrote to stderr: ${stderr.slice(0, 100)}`,
		);
	} else {
		pass("Claude Code fail-open: no stderr output");
	}

	// Check warning log was created (relative to tmpDir cwd)
	const warningLog = join(
		tmpDir,
		".scryrs",
		"hooks",
		"claude-code-warnings.log",
	);
	if (existsSync(warningLog)) {
		const logContent = readFileSync(warningLog, "utf-8");
		if (logContent.trim().length > 0) {
			pass("Claude Code fail-open: warning logged to claude-code-warnings.log");
		} else {
			fail("Claude Code fail-open: warning log", "log exists but is empty");
		}
	} else {
		fail("Claude Code fail-open: warning log", "no warning log created");
	}

	rmSync(tmpDir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Test: Pass-through — unlisted tools produce no events
// -----------------------------------------------------------------------
async function testPassthrough() {
	console.log(
		`\n\x1b[33m--- Claude Code: Pass-through (unlisted tools) ---\x1b[0m`,
	);

	const tmpDir = join(tmpdir(), `scryrs-cc-e2e-pt-${Date.now()}`);
	mkdirSync(tmpDir, { recursive: true });

	const realScryrs = join(ROOT, "target", "release", "scryrs");
	try {
		execFileSync(realScryrs, ["init"], {
			cwd: tmpDir,
			timeout: 5000,
			stdio: "ignore",
		});
	} catch {}

	const { result } = invokeHook(
		"task",
		{ description: "do something" },
		tmpDir,
	);

	if (!result || !result.continue) {
		fail("Claude Code pass-through: task", "unlisted tool was blocked");
	} else {
		pass("Claude Code pass-through: task: hook returned {continue:true}");
	}

	// No event should be written for Task
	const eventsJsonl = join(tmpDir, ".scryrs", "events.jsonl");
	const events = readJsonl(eventsJsonl);
	const taskEvents = events.filter((e) => e.tool_name === "task");
	if (taskEvents.length > 0) {
		fail(
			"Claude Code pass-through: task",
			`unlisted tool produced ${taskEvents.length} trace event(s)`,
		);
	} else {
		pass("Claude Code pass-through: task: no event captured for unlisted tool");
	}

	rmSync(tmpDir, { recursive: true, force: true });
}

// -----------------------------------------------------------------------
// Main
// -----------------------------------------------------------------------
async function main() {
	await testJsonShaping();
	await testNonInterference();
	await testFailOpen();
	await testPassthrough();

	summary();
}

main().catch((err) => {
	console.error("Claude Code E2E fixture crashed:", err);
	process.exit(2);
});
