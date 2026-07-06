#!/usr/bin/env node

/**
 * Mock live server + dashboard launcher for the Playwright container.
 *
 * 1. Starts mock-live-server.mjs and captures its address/pid
 * 2. Starts `scryrs dashboard` pointing at the mock SSE server
 * 3. Waits for the dashboard to be ready
 * 4. Exec's the Playwright test script
 * 5. Cleans up child processes on exit
 *
 * Usage: node scripts/verification/lib/mock-live-server-launcher.mjs
 */

import { spawn } from "node:child_process";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import net from "node:net";
import http from "node:http";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..", "..", "..");

const SCRYRS_BIN =
	process.env.SCRYRS_BIN || join(ROOT, "target", "release", "scryrs");
const DASHBOARD_PORT = 9090;
const SERVER_READY_TIMEOUT_MS = 30_000;
const MOCK_START_TIMEOUT_MS = 10_000;

// Collect child pids for cleanup.
const children = [];

function cleanup() {
	for (const pid of children) {
		try {
			process.kill(pid, "SIGTERM");
		} catch {
			/* already gone */
		}
	}
}

process.on("exit", cleanup);
process.on("SIGINT", () => {
	cleanup();
	process.exit(1);
});
process.on("SIGTERM", () => {
	cleanup();
	process.exit(1);
});

/**
 * Wait until a TCP port is accepting connections.
 */
async function waitForPort(host, port, timeoutMs) {
	const deadline = Date.now() + timeoutMs;
	while (Date.now() < deadline) {
		try {
			await new Promise((resolve, reject) => {
				const sock = net.createConnection({ host, port });
				sock.on("connect", () => {
					sock.destroy();
					resolve();
				});
				sock.on("error", reject);
				sock.setTimeout(500, () => {
					sock.destroy();
					reject(new Error("timeout"));
				});
			});
			return;
		} catch {
			await new Promise((r) => setTimeout(r, 200));
		}
	}
	throw new Error(`port ${host}:${port} not ready after ${timeoutMs}ms`);
}

/**
 * Wait until an HTTP endpoint returns a 2xx response.
 */
async function waitForHttp(url, timeoutMs) {
	const deadline = Date.now() + timeoutMs;
	while (Date.now() < deadline) {
		try {
			await new Promise((resolve, reject) => {
				http
					.get(url, (res) => {
						if (res.statusCode >= 200 && res.statusCode < 400) {
							res.resume();
							resolve();
						} else {
							res.resume();
							reject(new Error(`HTTP ${res.statusCode}`));
						}
					})
					.on("error", reject);
			});
			return;
		} catch {
			await new Promise((r) => setTimeout(r, 200));
		}
	}
	throw new Error(`HTTP ${url} not ready after ${timeoutMs}ms`);
}

// ── Start mock SSE server ─────────────────────────────────────────────────

console.log("Starting mock live SSE server...");
const mockServer = spawn("node", [join(__dirname, "mock-live-server.mjs")], {
	stdio: ["ignore", "pipe", "pipe"],
	env: { ...process.env, REPLAY_COUNT: "10" },
	cwd: ROOT,
});
children.push(mockServer.pid);

// Read the MOCK_LIVE_SERVER line from stdout.
let mockUrl = "";
const mockStdoutChunks = [];
mockServer.stdout.on("data", (chunk) => {
	mockStdoutChunks.push(chunk);
	const text = Buffer.concat(mockStdoutChunks).toString();
	const match = text.match(/^MOCK_LIVE_SERVER=(http:\/\/[^\s]+)$/m);
	if (match) {
		mockUrl = match[1];
	}
});
mockServer.stderr.on("data", (chunk) =>
	process.stderr.write(`[mock-server] ${chunk}`),
);

// Wait for the mock URL.
const mockDeadline = Date.now() + MOCK_START_TIMEOUT_MS;
while (!mockUrl && Date.now() < mockDeadline) {
	await new Promise((r) => setTimeout(r, 100));
}
if (!mockUrl) {
	console.error(
		"ERROR: mock server did not emit MOCK_LIVE_SERVER within timeout",
	);
	cleanup();
	process.exit(1);
}
console.log(`  Mock SSE server: ${mockUrl}`);

// ── Start scryrs dashboard ────────────────────────────────────────────────

console.log("Starting scryrs dashboard...");
const dashboardHost = "http://127.0.0.1:" + DASHBOARD_PORT;
const dashboard = spawn(
	SCRYRS_BIN,
	[
		"dashboard",
		"--port",
		String(DASHBOARD_PORT),
		"--bind",
		"127.0.0.1",
		"--server-url",
		mockUrl,
		"--repository-id",
		"repo-a",
		"--no-open",
	],
	{
		stdio: ["ignore", "pipe", "pipe"],
		cwd: ROOT,
	},
);
children.push(dashboard.pid);

dashboard.stdout.on("data", (chunk) =>
	process.stderr.write(`[dashboard] ${chunk}`),
);
dashboard.stderr.on("data", (chunk) =>
	process.stderr.write(`[dashboard:err] ${chunk}`),
);

dashboard.on("exit", (code) => {
	if (code !== 0 && code !== null) {
		console.error(`ERROR: dashboard exited with code ${code}`);
	}
});

// Wait for dashboard to be ready.
try {
	await waitForPort("127.0.0.1", DASHBOARD_PORT, SERVER_READY_TIMEOUT_MS);
	await waitForHttp(dashboardHost, SERVER_READY_TIMEOUT_MS);
} catch (err) {
	console.error(`ERROR: dashboard failed to become ready: ${err.message}`);
	cleanup();
	process.exit(1);
}
console.log(`  Dashboard ready: ${dashboardHost}`);

// ── Run Playwright test ───────────────────────────────────────────────────

console.log("Running Playwright browser smoke...");
const specPath = join(
	ROOT,
	"scripts",
	"verification",
	"live-dashboard-smoke.spec.cjs",
);

const playwrightTest = spawn("node", [specPath], {
	stdio: "inherit",
	env: {
		...process.env,
		MOCK_LIVE_URL: mockUrl,
		DASHBOARD_URL: dashboardHost,
		REPLAY_COUNT: "10",
	},
	cwd: ROOT,
});

playwrightTest.on("exit", (code) => {
	cleanup();
	process.exit(code ?? 1);
});
