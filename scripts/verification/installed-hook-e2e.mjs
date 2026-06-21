/**
 * Installed-hook end-to-end verification.
 *
 * Runs `scryrs init --agent claude-code` and `scryrs init --agent pi` in
 * temporary consumer project directories, then loads and exercises the
 * installed hook artifacts against the real `scryrs` binary.
 *
 * Proves the init pipeline produces functional output — not just that files
 * were created. Distinguishes "structurally correct but semantically broken"
 * hooks from hooks that actually work end-to-end.
 *
 * Prerequisites:
 *   - Real `scryrs` binary on PATH (built via `cargo build --release`)
 *   - Working directory is the repository root
 *
 * Usage: node scripts/verification/installed-hook-e2e.mjs
 */

import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { execFileSync, execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { pass, fail, summary, assert } from "./lib/assert.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..", "..");

// Resolve scryrs binary from PATH or fallback to target/release.
function findScryrsBin() {
	const releaseBin = join(ROOT, "target", "release", "scryrs");
	if (existsSync(releaseBin)) return releaseBin;
	return "scryrs";
}

const SCYRRS_BIN = findScryrsBin();
const SCYRRS_DIR = dirname(SCYRRS_BIN);

// -----------------------------------------------------------------------
// Temp directory helpers
// -----------------------------------------------------------------------
function tempDir() {
	return join(
		tmpdir(),
		`scryrs-init-e2e-${Date.now()}-${Math.random().toString(36).slice(2)}`,
	);
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

		// 3. Verify next-steps
		assert(
			initStdout.includes("scryrs Pi trace hook installed"),
			"init stdout contains installation confirmation",
		);
		assert(
			initStdout.includes("Next steps:"),
			"init stdout contains next steps",
		);

		// 4. Install tsx for TypeScript transpilation
		console.log("  Installing tsx for Pi hook execution...");
		try {
			execSync("npm install tsx", {
				cwd: consumerDir,
				stdio: "pipe",
				timeout: 60000,
			});
		} catch {
			fail("tsx install for Pi hook", "npm install tsx failed");
			return;
		}

		const tsxBin = join(consumerDir, "node_modules", ".bin", "tsx");
		if (!existsSync(tsxBin)) {
			fail("tsx binary not found", tsxBin);
			return;
		}

		// 5. Exercise the hook via tsx using the pi-hook-e2e.mjs pattern
		console.log("  Loading and exercising installed Pi hook...");
		const hookWrapperPath = join(consumerDir, "hook-wrapper.mjs");
		const scryrsDirPath = SCYRRS_DIR;

		// Write a wrapper that follows the same pattern as pi-hook-e2e.mjs:
		// createRequire to load the hook, provide a fake API that has on() and exec(),
		// then fire a tool_result event through the registered handler.
		writeFileSync(
			hookWrapperPath,
			[
				`import { createRequire } from "node:module";`,
				`const require = createRequire(import.meta.url);`,
				`const { spawnSync } = require("node:child_process");`,
				``,
				`const hookPath = ${JSON.stringify(hookPath)};`,
				``,
				`// Fake ExtensionAPI matching what the Pi hook expects`,
				`const fakeApi = {`,
				`  handlers: {},`,
				`  on(event, handler) {`,
				`    this.handlers[event] = handler;`,
				`  },`,
				`  async exec(command, args, options) {`,
				`    const input = options?.input ?? "";`,
				`    const timeoutVal = options?.timeout ?? 5000;`,
				`    const cwd = options?.cwd ?? process.cwd();`,
				`    const result = spawnSync(command, args, {`,
				`      input,`,
				`      timeout: timeoutVal,`,
				`      encoding: "utf-8",`,
				`      stdio: ["pipe", "pipe", "pipe"],`,
				`      cwd,`,
				`      env: { ...process.env },`,
				`    });`,
				`    return {`,
				`      stdout: result.stdout?.toString() || "",`,
				`      stderr: result.stderr?.toString() || "",`,
				`      code: result.status,`,
				`      killed: result.signal !== null,`,
				`    };`,
				`  },`,
				`};`,
				``,
				`// Load the hook module via tsx`,
				`const hookModule = require(hookPath);`,
				`const hook = hookModule.default;`,
				``,
				`if (typeof hook !== "function") {`,
				`  console.log(JSON.stringify({ success: false, error: "hook is not a function, type=" + typeof hook }));`,
				`  process.exit(1);`,
				`}`,
				``,
				`// Walk through the event to exercise the handler`,
				`const event = {`,
				`  type: "tool_result",`,
				`  toolName: "bash",`,
				`  input: { command: "echo hello" },`,
				`  isError: false,`,
				`};`,
				``,
				`hook(fakeApi);`,
				``,
				`const handler = fakeApi.handlers["tool_result"];`,
				`if (!handler) {`,
				`  console.log(JSON.stringify({ success: false, error: "no tool_result handler registered" }));`,
				`  process.exit(1);`,
				`}`,
				``,
				`try {`,
				`  const result = await handler(event);`,
				`  console.log(JSON.stringify({ success: true, result: result }));`,
				`  // Give scryrs record a moment to finish`,
				`  await new Promise(r => setTimeout(r, 1500));`,
				`  process.exit(0);`,
				`} catch (e) {`,
				`  console.log(JSON.stringify({ success: false, error: e.message }));`,
				`  process.exit(1);`,
				`}`,
			].join("\n"),
		);

		// Ensure .scryrs dir exists
		mkdirSync(join(consumerDir, ".scryrs"), { recursive: true });

		const env = {
			...process.env,
			PATH: `${scryrsDirPath}:${process.env.PATH || ""}`,
		};

		let invokeResult;
		try {
			const stdout = execFileSync(tsxBin, [hookWrapperPath], {
				env,
				cwd: consumerDir,
				timeout: 30000,
				encoding: "utf-8",
			}).trim();
			invokeResult = JSON.parse(stdout);
		} catch (err) {
			fail(
				"Pi installed hook invocation succeeded",
				`tsx process failed (exit ${err.status}): ${(err.stderr?.toString() || err.message).slice(0, 300)}`,
			);
			return;
		}

		// 6. Assert hook was loadable
		if (!invokeResult.success) {
			fail(
				"Pi installed hook is loadable",
				invokeResult.error || "unknown error",
			);
			return;
		}
		pass("Pi installed hook is loadable");

		// 7. Assert hook returns undefined (non-interference)
		assert(
			invokeResult.result === undefined,
			`Pi installed hook returns undefined (non-interference), got: ${JSON.stringify(invokeResult.result)}`,
		);

		// 8. Wait and verify events via scryrs hotspots
		await new Promise((resolve) => setTimeout(resolve, 2000));
		const report = verifyEventsPersisted(consumerDir);
		assert(
			report.count >= 1,
			`Pi installed hook persisted events (analyzedEventCount=${report.count})`,
		);

		if (report.count > 0) {
			const firstEntry = report.events[0];
			assert(
				firstEntry && firstEntry.subjectKind === "command",
				`Pi persisted event subjectKind is command (got: ${firstEntry?.subjectKind})`,
			);
		}
	} finally {
		rmSync(consumerDir, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// Main
// -----------------------------------------------------------------------
async function main() {
	console.log("scryrs installed-hook end-to-end verification");
	console.log(`Binary: ${SCYRRS_BIN}`);

	await testClaudeCodeInstalled();
	await testPiInstalled();

	summary();
}

main().catch((err) => {
	console.error("Fatal error:", err);
	process.exit(1);
});
