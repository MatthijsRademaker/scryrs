/**
 * Installed-hook end-to-end verification.
 *
 * Runs `scryrs init --agent claude-code` and `scryrs init --agent pi` in
 * temporary consumer project directories, then proves the installed artifacts
 * actually capture events against the real `scryrs` binary.
 *
 *  - Claude Code: init create-or-merges `.claude/settings.json` with the native
 *    `scryrs hook claude-code` command hook (no `.mjs`, no node hook). The
 *    fixture drives `scryrs hook claude-code` with a PreToolUse payload on
 *    stdin and confirms persistence via `scryrs hotspots .`.
 *  - Pi: init installs the slimmed `index.ts` shim. The fixture transpiles it
 *    via tsx, exercises it with a simulated `tool_result`, proves it invokes
 *    `scryrs hook pi --file`, and confirms persistence via `scryrs hotspots .`.
 *
 * Prerequisites:
 *   - Real `scryrs` binary (SCRYRS_BIN env, or target/release/scryrs)
 *   - tsx and better-sqlite3 available (npm install)
 *
 * Usage: node scripts/verification/installed-hook-e2e.mjs [--claude-only|--pi-only]
 */

import {
	existsSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import { pass, fail, summary, assert } from "./lib/assert.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..", "..");

function findScryrsBin() {
	if (process.env.SCRYRS_BIN) return process.env.SCRYRS_BIN;
	const releaseBin = join(ROOT, "target", "release", "scryrs");
	if (existsSync(releaseBin)) return releaseBin;
	return "scryrs";
}

const SCRYRS_BIN = findScryrsBin();
const TSX = join(ROOT, "node_modules", ".bin", "tsx");
const SHIM_DRIVER = join(__dirname, "lib", "pi-shim-driver.mjs");

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
		throw new Error("--claude-only and --pi-only are mutually exclusive");
	}
	return { runClaude, runPi };
}

function tempDir() {
	return mkdtempSync(join(tmpdir(), "scryrs-init-e2e-"));
}

/** Verify persistence by running `scryrs hotspots .` and reading analyzedEventCount. */
function analyzedEventCount(cwd) {
	try {
		const stdout = execFileSync(SCRYRS_BIN, ["hotspots", "."], {
			cwd,
			encoding: "utf-8",
			timeout: 5000,
		});
		const report = JSON.parse(stdout);
		return report.runMetadata?.analyzedEventCount || 0;
	} catch {
		return 0;
	}
}

function countNativeClaudeHook(settings) {
	const pre = settings?.hooks?.PreToolUse;
	if (!Array.isArray(pre)) return 0;
	let n = 0;
	for (const entry of pre) {
		const hooks = entry?.hooks;
		if (!Array.isArray(hooks)) continue;
		for (const h of hooks) {
			if (h?.type === "command" && h?.command === "scryrs hook claude-code") n++;
		}
	}
	return n;
}

// -----------------------------------------------------------------------
// Claude Code installed-hook test
// -----------------------------------------------------------------------
function testClaudeCodeInstalled() {
	console.log("\n\x1b[33m--- Claude Code: Installed-hook E2E ---\x1b[0m");
	const consumerDir = tempDir();
	try {
		// 1. Run scryrs init.
		const initStdout = execFileSync(SCRYRS_BIN, ["init", "--agent", "claude-code"], {
			cwd: consumerDir,
			encoding: "utf-8",
			timeout: 5000,
		});

		// 2. settings.json has the native command hook; no .mjs file anywhere.
		const settingsPath = join(consumerDir, ".claude", "settings.json");
		assert(existsSync(settingsPath), "init creates .claude/settings.json");
		const settings = JSON.parse(readFileSync(settingsPath, "utf-8"));
		assert(
			countNativeClaudeHook(settings) === 1,
			"settings.json PreToolUse hook command is `scryrs hook claude-code`",
		);
		assert(
			!existsSync(join(consumerDir, ".claude", "hooks")),
			"no .claude/hooks dir (no .mjs file) is created",
		);

		// 3. Next-step text references the native command, never a .mjs.
		assert(
			initStdout.includes("scryrs hook claude-code"),
			"next steps mention `scryrs hook claude-code`",
		);
		assert(initStdout.includes("settings.json"), "next steps mention settings.json");
		assert(
			initStdout.includes("Restart your Claude Code session"),
			"next steps mention restarting the session",
		);
		assert(!initStdout.toLowerCase().includes(".mjs"), "next steps do not mention any .mjs file");

		// 4. Drive the native command with a PreToolUse payload on stdin.
		const payload = JSON.stringify({
			session_id: "installed-cc",
			cwd: consumerDir,
			tool_name: "Read",
			tool_input: { file_path: "src/main.rs" },
		});
		const r = spawnSync(SCRYRS_BIN, ["hook", "claude-code"], {
			input: payload,
			cwd: consumerDir,
			encoding: "utf-8",
			timeout: 15000,
		});
		assert(r.status === 0 && (r.stdout ?? "") === "", "native hook exits 0 with empty stdout");

		// 5. Confirm persistence via hotspots.
		const count = analyzedEventCount(consumerDir);
		assert(count >= 1, `installed Claude Code integration persisted events (analyzedEventCount=${count})`);
	} catch (err) {
		fail("Claude Code installed E2E", String(err?.message || err));
	} finally {
		rmSync(consumerDir, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// Pi installed-hook test
// -----------------------------------------------------------------------
function testPiInstalled() {
	console.log("\n\x1b[33m--- Pi: Installed-hook E2E ---\x1b[0m");
	if (!existsSync(TSX)) {
		fail("tsx available", `not found at ${TSX} — run npm install`);
		return;
	}
	const consumerDir = tempDir();
	try {
		// 1. Run scryrs init.
		const initStdout = execFileSync(SCRYRS_BIN, ["init", "--agent", "pi"], {
			cwd: consumerDir,
			encoding: "utf-8",
			timeout: 5000,
		});

		// 2. Installed artifact exists at the expected path.
		const hookPath = join(consumerDir, ".pi", "extensions", "pi-trace", "index.ts");
		assert(existsSync(hookPath), "installed Pi hook exists at .pi/extensions/pi-trace/index.ts");
		if (!existsSync(hookPath)) return;

		const src = readFileSync(hookPath, "utf-8");
		assert(src.includes("hook"), "installed Pi shim delegates to `scryrs hook pi`");
		assert(!src.includes("scryrs record"), "installed Pi shim no longer uses `scryrs record`");

		// 3. Next-step text.
		assert(initStdout.includes("Next steps:"), "init stdout contains next steps");
		assert(initStdout.includes("Reload Pi"), "next steps instruct reloading Pi");

		// 4. Transpile via tsx and exercise with a simulated tool_result; the shim
		//    must invoke `scryrs hook pi --file` and persist.
		writeFileSync(join(consumerDir, "README.md"), "# installed fixture\n");
		const driver = spawnSync(TSX, [SHIM_DRIVER], {
			cwd: consumerDir,
			env: {
				...process.env,
				PI_HOOK_PATH: hookPath,
				SCRYRS_BIN,
				STORE_DIR: consumerDir,
				FAIL_OPEN: "0",
			},
			encoding: "utf-8",
			timeout: 30000,
		});
		assert(driver.status === 0, "installed Pi shim transpiles and runs via tsx without error");

		let parsed = null;
		try {
			parsed = JSON.parse((driver.stdout ?? "").trim().split("\n").pop());
		} catch {
			/* leave null */
		}
		const delegated = (parsed?.calls || []).some(
			(c) =>
				c.cmd === "scryrs" &&
				c.args[0] === "hook" &&
				c.args[1] === "pi" &&
				c.args[2] === "--file",
		);
		assert(delegated, "installed Pi shim invokes `scryrs hook pi --file <tmp>`");

		// 5. Confirm persistence via hotspots.
		const count = analyzedEventCount(consumerDir);
		assert(count >= 1, `installed Pi integration persisted events (analyzedEventCount=${count})`);
	} catch (err) {
		fail("Pi installed E2E", String(err?.message || err));
	} finally {
		rmSync(consumerDir, { recursive: true, force: true });
	}
}

// -----------------------------------------------------------------------
// Main
// -----------------------------------------------------------------------
console.log("scryrs installed-hook end-to-end verification");
console.log(`Binary: ${SCRYRS_BIN}`);

const mode = parseMode(process.argv.slice(2));
if (mode.runClaude) testClaudeCodeInstalled();
if (mode.runPi) testPiInstalled();
summary();
