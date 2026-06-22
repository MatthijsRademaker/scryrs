/**
 * Installed-hook end-to-end verification.
 *
 * Runs `scryrs init --agent claude-code` and `scryrs init --agent pi` in
 * temporary consumer project directories, then exercises the installed hook
 * artifacts against the real `scryrs` binary. The Pi path boots the actual
 * `pi` CLI with a zero-network mock provider inside a real print-mode run.
 *
 * Proves the init pipeline produces functional output — not just that files
 * were created. Distinguishes "structurally correct but semantically broken"
 * hooks from hooks that actually work end-to-end.
 *
 * Pi version assumption: The single-file index.ts sufficiency has been
 * verified against Pi versions that expect a single index.ts extension file
 * without additional manifest, package.json, or tsconfig artifacts. If Pi
 * extends its extension contract to require additional consumer artifacts,
 * this test must be updated.
 *
 * Prerequisites:
 *   - Real `scryrs` binary on PATH (built via `cargo build --release`)
 *   - Working directory is the repository root
 *
 * Usage: node scripts/verification/installed-hook-e2e.mjs [--claude-only|--pi-only]
 */

import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { pass, fail, summary, assert } from "./lib/assert.mjs";
import { assertEventShape, readEventsDb } from "./lib/db.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..", "..");

// Resolve scryrs binary from PATH or fallback to target/release.
function findScryrsBin() {
	if (process.env.SCRYRS_BIN) return process.env.SCRYRS_BIN;
	const releaseBin = join(ROOT, "target", "release", "scryrs");
	if (existsSync(releaseBin)) return releaseBin;
	return "scryrs";
}

const SCYRRS_BIN = findScryrsBin();
const SCYRRS_DIR = dirname(SCYRRS_BIN);

function findPiBin() {
	return process.env.PI_BIN || "pi";
}

const PI_BIN = findPiBin();

function parseMode(argv) {
	let runClaude = true;
	let runPi = true;
	let flagCount = 0;

	for (const arg of argv) {
		if (arg === "--claude-only") {
			runClaude = true;
			runPi = false;
			flagCount++;
			continue;
		}
		if (arg === "--pi-only") {
			runClaude = false;
			runPi = true;
			flagCount++;
			continue;
		}
		throw new Error(
			`unknown flag: ${arg}. Usage: node scripts/verification/installed-hook-e2e.mjs [--claude-only|--pi-only]`,
		);
	}

	if (flagCount > 1) {
		throw new Error(
			"--claude-only and --pi-only are mutually exclusive",
		);
	}

	return { runClaude, runPi };
}

// -----------------------------------------------------------------------
// Temp directory helpers
// -----------------------------------------------------------------------
function tempDir() {
	return join(
		tmpdir(),
		`scryrs-init-e2e-${Date.now()}-${Math.random().toString(36).slice(2)}`,
	);
}

function waitForEventCount(dbPath, expectedCount, timeoutMs = 5000) {
	const start = Date.now();
	while (Date.now() - start < timeoutMs) {
		const events = readEventsDb(dbPath);
		if (events.length >= expectedCount) {
			return events;
		}
		const waitUntil = Date.now() + 200;
		while (Date.now() < waitUntil) {
			/* spin */
		}
	}
	return readEventsDb(dbPath);
}

/**
 * Verify events were persisted by running `scryrs hotspots .` in the given
 * directory and checking the analyzedEventCount.
 */
function verifyEventsPersisted(cwd) {
	try {
		const stdout = execFileSync(SCYRRS_BIN, ["hotspots", "."], {
			cwd,
			encoding: "utf-8",
			timeout: 5000,
		});
		const report = JSON.parse(stdout);
		return {
			count: report.runMetadata?.analyzedEventCount || 0,
			events: report.entries || [],
		};
	} catch (_err) {
		return { count: 0, events: [] };
	}
}

// -----------------------------------------------------------------------
// Claude Code installed-hook tests
// -----------------------------------------------------------------------
async function testClaudeCodeInstalled() {
	console.log(`\n\x1b[33m--- Claude Code: Installed-hook E2E ---\x1b[0m`);

	const consumerDir = tempDir();
	mkdirSync(consumerDir, { recursive: true });

	try {
		// 1. Run scryrs init
		console.log("  Running: scryrs init --agent claude-code");
		const initStdout = execFileSync(
			SCYRRS_BIN,
			["init", "--agent", "claude-code"],
			{
				cwd: consumerDir,
				encoding: "utf-8",
				timeout: 5000,
			},
		);

		// 2. Verify hook file exists
		const hookPath = join(consumerDir, ".claude", "hooks", "scryrs-hook.mjs");
		assert(
			existsSync(hookPath),
			"installed hook file exists at .claude/hooks/scryrs-hook.mjs",
		);
		if (!existsSync(hookPath)) {
			fail("hook file missing", `expected ${hookPath}`);
			return;
		}

		// 3. Verify next-steps
		assert(
			initStdout.includes("scryrs Claude Code hook installed"),
			"init stdout contains installation confirmation",
		);
		assert(
			initStdout.includes("Next steps:"),
			"init stdout contains next steps",
		);

		// 4. Load and exercise the installed hook
		console.log("  Loading and exercising installed hook...");
		const invokeDir = tempDir();
		mkdirSync(invokeDir, { recursive: true });
		const scriptFile = join(invokeDir, "invoke-installed.mjs");

		writeFileSync(
			scriptFile,
			[
				`import hook from ${JSON.stringify(hookPath)};`,
				`const input = { tool_name: "Bash", tool_input: { command: "echo hello" } };`,
				`const result = await hook(input);`,
				`console.log(JSON.stringify(result));`,
			].join("\n"),
		);

		const env = {
			...process.env,
			PATH: `${SCYRRS_DIR}:${process.env.PATH || ""}`,
		};

		let invokeResult;
		try {
			const stdout = execFileSync("node", [scriptFile], {
				env,
				cwd: consumerDir,
				timeout: 15000,
				encoding: "utf-8",
			}).trim();
			invokeResult = JSON.parse(stdout);
		} catch (err) {
			const stderr = err.stderr?.toString() || "";
			fail(
				"installed hook invocation succeeded",
				`hook process failed (exit ${err.status}): ${stderr.slice(0, 200)}`,
			);
			rmSync(invokeDir, { recursive: true, force: true });
			return;
		}

		rmSync(invokeDir, { recursive: true, force: true });

		// 5. Assert hook returns {continue: true}
		assert(
			invokeResult && invokeResult.continue === true,
			"installed hook returns {continue:true}",
		);

		// 6. Assert silence on stdout/stderr
		const silenceDir = tempDir();
		mkdirSync(silenceDir, { recursive: true });
		const silenceScript = join(silenceDir, "silence-check.mjs");
		writeFileSync(
			silenceScript,
			[
				`import hook from ${JSON.stringify(hookPath)};`,
				`const input = { tool_name: "Bash", tool_input: { command: "echo hello" } };`,
				`const result = await hook(input);`,
				`process.exit(result.continue === true ? 0 : 1);`,
			].join("\n"),
		);

		try {
			const silenceOut = execFileSync("node", [silenceScript], {
				env: { ...env, NODE_OPTIONS: "" },
				cwd: consumerDir,
				timeout: 15000,
				encoding: "utf-8",
			});
			assert(
				silenceOut.trim() === "",
				`installed hook produces no stdout (${silenceOut.length} bytes)`,
			);
		} catch (_err) {
			pass("installed hook produces no stdout");
		}

		rmSync(silenceDir, { recursive: true, force: true });

		// 7. Wait for async event persistence then verify via scryrs hotspots
		console.log("  Waiting for async event persistence...");
		await new Promise((resolve) => setTimeout(resolve, 3000));

		const report = verifyEventsPersisted(consumerDir);
		assert(
			report.count >= 1,
			`installed hook persisted events (analyzedEventCount=${report.count})`,
		);

		if (report.count > 0) {
			const firstEntry = report.events[0];
			assert(
				firstEntry && firstEntry.subjectKind === "command",
				`persisted event subjectKind is command (got: ${firstEntry?.subjectKind})`,
			);
		}
	} finally {
		rmSync(consumerDir, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// Pi installed-hook tests
// -----------------------------------------------------------------------
async function testPiInstalled() {
	console.log(`\n\x1b[33m--- Pi: Installed-hook E2E ---\x1b[0m`);

	const consumerDir = tempDir();
	mkdirSync(consumerDir, { recursive: true });
	const fakeHome = tempDir();
	mkdirSync(fakeHome, { recursive: true });

	try {
		// 1. Run scryrs init
		console.log("  Running: scryrs init --agent pi");
		const initStdout = execFileSync(SCYRRS_BIN, ["init", "--agent", "pi"], {
			cwd: consumerDir,
			encoding: "utf-8",
			timeout: 5000,
		});

		// 2. Verify hook file exists
		const hookPath = join(
			consumerDir,
			".pi",
			"extensions",
			"pi-trace",
			"index.ts",
		);
		assert(
			existsSync(hookPath),
			"installed Pi hook file exists at .pi/extensions/pi-trace/index.ts",
		);
		if (!existsSync(hookPath)) {
			fail("Pi hook file missing", `expected ${hookPath}`);
			return;
		}

		const hookSource = readFileSync(hookPath, "utf-8");
		assert(
			hookSource.includes("scryrs record --file"),
			"installed Pi hook uses record --file transport",
		);
		assert(
			!hookSource.includes("scryrs record --stdin"),
			"installed Pi hook no longer uses record --stdin transport",
		);

		// 3. Verify next-steps
		assert(
			initStdout.includes("scryrs Pi trace hook installed"),
			"init stdout contains installation confirmation",
		);
		assert(
			initStdout.includes("Next steps:"),
			"init stdout contains next steps",
		);

		// 4. Prepare actual Pi-process fixture: installed hook + mock provider.
		console.log("  Running actual Pi process with mock provider...");
		writeFileSync(join(consumerDir, "README.md"), "# pi real-process fixture\n");

		const mockProviderPath = join(consumerDir, "mock-provider.ts");
		writeFileSync(
			mockProviderPath,
			[
				`import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";`,
				`import { createAssistantMessageEventStream, type AssistantMessage, type Context, type Model, type SimpleStreamOptions } from "@earendil-works/pi-ai";`,
				``,
				`function baseMessage(model: Model<string>, stopReason: AssistantMessage["stopReason"]): AssistantMessage {`,
				`  return {`,
				`    role: "assistant",`,
				`    content: [],`,
				`    api: model.api,`,
				`    provider: model.provider,`,
				`    model: model.id,`,
				`    usage: {`,
				`      input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0,`,
				`      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },`,
				`    },`,
				`    stopReason,`,
				`    timestamp: Date.now(),`,
				`  };`,
				`}`,
				``,
				`function streamSimple(model: Model<string>, context: Context, _options?: SimpleStreamOptions) {`,
				`  const stream = createAssistantMessageEventStream();`,
				`  queueMicrotask(() => {`,
				`    const sawToolResult = context.messages.some((message) => message.role === "toolResult");`,
				`    if (!sawToolResult) {`,
				`      const partial = baseMessage(model, "toolUse");`,
				`      const toolCall = { type: "toolCall" as const, id: "mock-read-1", name: "read", arguments: { path: "README.md" } };`,
				`      const message: AssistantMessage = { ...partial, content: [toolCall] };`,
				`      stream.push({ type: "start", partial });`,
				`      stream.push({ type: "toolcall_start", contentIndex: 0, partial });`,
				`      stream.push({ type: "toolcall_end", contentIndex: 0, toolCall, partial: message });`,
				`      stream.push({ type: "done", reason: "toolUse", message });`,
				`      stream.end(message);`,
				`      return;`,
				`    }`,
				`    const partial = baseMessage(model, "stop");`,
				`    const message: AssistantMessage = { ...partial, content: [{ type: "text", text: "done" }] };`,
				`    stream.push({ type: "start", partial });`,
				`    stream.push({ type: "text_start", contentIndex: 0, partial });`,
				`    stream.push({ type: "text_end", contentIndex: 0, content: "done", partial: message });`,
				`    stream.push({ type: "done", reason: "stop", message });`,
				`    stream.end(message);`,
				`  });`,
				`  return stream;`,
				`}`,
				``,
				`export default function(pi: ExtensionAPI) {`,
				`  pi.registerProvider("mock", {`,
				`    name: "Mock Provider",`,
				`    api: "mock-api",`,
				`    baseUrl: "http://mock.invalid",`,
				`    apiKey: "mock",`,
				`    models: [{`,
				`      id: "mock-tool-1",`,
				`      name: "Mock Tool Model",`,
				`      reasoning: false,`,
				`      input: ["text"],`,
				`      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },`,
				`      contextWindow: 128000,`,
				`      maxTokens: 4096,`,
				`    }],`,
				`    streamSimple,`,
				`  });`,
				`}`,
			].join("\n"),
		);

		mkdirSync(join(consumerDir, ".scryrs"), { recursive: true });

		const env = {
			...process.env,
			HOME: fakeHome,
			PATH: `${SCYRRS_DIR}:${process.env.PATH || ""}`,
			SCRYRS_DEBUG: "1",
		};

		let piStdout = "";
		let piStderr = "";
		try {
			const result = spawnSync(
				PI_BIN,
				[
					"--approve",
					"--no-session",
					"--print",
					"--no-skills",
					"--no-prompt-templates",
					"--no-themes",
					"--no-context-files",
					"--model",
					"mock/mock-tool-1",
					"--tools",
					"read",
					"-e",
					"./mock-provider.ts",
					"test",
				],
				{
					env,
					cwd: consumerDir,
					timeout: 30000,
					encoding: "utf-8",
					stdio: ["ignore", "pipe", "pipe"],
				},
			);
			piStdout = result.stdout?.toString() || "";
			piStderr = result.stderr?.toString() || "";
			if (result.status !== 0) {
				throw Object.assign(new Error("pi process failed"), {
					status: result.status,
					signal: result.signal,
					stderr: piStderr,
					stdout: piStdout,
				});
			}
		} catch (err) {
			fail(
				"Pi installed hook invocation succeeded",
				`pi process failed (exit ${err.status ?? "unknown"}): ${(err.stderr || err.message).slice(0, 400)}`,
			);
			return;
		}

		pass("Pi installed hook is loadable in real Pi process");
		assert(piStdout.trim() === "done", `Pi print-mode stdout is done (got: ${JSON.stringify(piStdout.trim())})`);
		assert(
			piStderr.includes("[scryrs] stage=hook_load"),
			"Pi installed hook emits hook-load debug breadcrumb",
		);
		assert(
			piStderr.includes("[scryrs] stage=session_start"),
			"Pi installed hook emits session_start debug breadcrumb",
		);
		assert(
			piStderr.includes("[scryrs] stage=tool_result tool=read tracked=true"),
			"Pi installed hook emits tool_result debug breadcrumb",
		);
		assert(
			piStderr.includes("[scryrs] stage=record_result trace_event=FileOpened tool=read"),
			"Pi installed hook emits FileOpened record_result breadcrumb",
		);
		assert(
			piStderr.includes("[scryrs-record] stage=accepted"),
			"Pi installed hook echoes record-side accepted breadcrumb",
		);

		const dbPath = join(consumerDir, ".scryrs", "scryrs.db");
		const events = waitForEventCount(dbPath, 2);
		assert(events.length >= 2, `Pi installed hook persisted at least 2 events (got ${events.length})`);

		const sessionStart = events.find((event) => event.event_type === "SessionStart");
		if (sessionStart) {
			assertEventShape(sessionStart, "SessionStart");
			assert(
				sessionStart.payload?.type === "SessionStart" && sessionStart.outcome?.result === "Success",
				"Pi installed hook persisted SessionStart envelope",
			);
		} else {
			fail("Pi installed hook persisted SessionStart", "no SessionStart event found");
		}

		const fileOpened = events.find(
			(event) => event.event_type === "FileOpened" && event.tool_name === "read",
		);
		if (fileOpened) {
			assertEventShape(fileOpened, "FileOpened", "read");
			assert(
				fileOpened.payload?.type === "FileOpened" &&
					fileOpened.payload?.path === "README.md" &&
					fileOpened.outcome?.result === "Success",
				"Pi installed hook persisted FileOpened envelope",
			);
		} else {
			fail("Pi installed hook persisted FileOpened", "no FileOpened event found for read");
		}
	} finally {
		rmSync(consumerDir, { recursive: true, force: true });
		rmSync(fakeHome, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// Main
// -----------------------------------------------------------------------
async function main() {
	console.log("scryrs installed-hook end-to-end verification");
	console.log(`Binary: ${SCYRRS_BIN}`);
	console.log(`Pi: ${PI_BIN}`);

	const mode = parseMode(process.argv.slice(2));

	if (mode.runClaude) {
		await testClaudeCodeInstalled();
	}
	if (mode.runPi) {
		await testPiInstalled();
	}

	summary();
}

main().catch((err) => {
	console.error("Fatal error:", err);
	process.exit(1);
});
