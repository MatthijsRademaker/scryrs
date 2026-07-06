#!/usr/bin/env node

/**
 * Live dashboard browser smoke — Playwright verification fixture (CJS).
 *
 * Proves the rendered live Signals view against a deterministic mock SSE
 * upstream: replay/live distinction, reconnect duplicate suppression,
 * and the reduced-motion code path.
 *
 * Prerequisites (supplied by orchestration):
 *   - MOCK_LIVE_URL env var — the mock SSE server address
 *   - DASHBOARD_URL env var — the scryrs dashboard server address
 *   - Running inside mcr.microsoft.com/playwright Docker image
 *   - playwright-core module available (via NODE_PATH)
 *
 * Usage: node scripts/verification/live-dashboard-smoke.spec.cjs
 */

"use strict";

const { chromium } = require("playwright-core");
const http = require("node:http");

// ── Inline assertion helpers (avoids CJS/ESM cross-import issues) ───────────

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

function assert(condition, name) {
	if (condition) {
		pass(name);
	} else {
		fail(name, "assertion failed");
	}
}

function summary() {
	console.log("");
	console.log("============================================");
	console.log(
		`Results: \x1b[32m${PASSED} passed\x1b[0m, \x1b[31m${FAILED} failed\x1b[0m, ${PASSED + FAILED} total`,
	);
	console.log("============================================");
	if (FAILED > 0) process.exit(1);
}

// ── Configuration ──────────────────────────────────────────────────────────

const MOCK_LIVE_URL = process.env.MOCK_LIVE_URL;
const DASHBOARD_URL = process.env.DASHBOARD_URL;
const REPLAY_COUNT = parseInt(process.env.REPLAY_COUNT || "10", 10);
const REPLAY_WAIT_MS = 10_000;
const LIVE_WAIT_MS = 5_000;

if (!MOCK_LIVE_URL || !DASHBOARD_URL) {
	console.error("MOCK_LIVE_URL and DASHBOARD_URL env vars are required");
	process.exit(2);
}

// ── Helpers ────────────────────────────────────────────────────────────────

async function waitForRows(page, minCount, timeoutMs) {
	const deadline = Date.now() + timeoutMs;
	while (Date.now() < deadline) {
		const count = await page.locator("[data-signal-id]").count();
		if (count >= minCount) return count;
		await page.waitForTimeout(200);
	}
	return page.locator("[data-signal-id]").count();
}

async function collectSignalIds(page) {
	return page
		.locator("[data-signal-id]")
		.evaluateAll((els) =>
			els.map((el) => parseInt(el.getAttribute("data-signal-id"), 10)),
		);
}

async function collectSignalPhases(page) {
	return page
		.locator("[data-signal-id]")
		.evaluateAll((els) =>
			els.map((el) => el.getAttribute("data-signal-phase")),
		);
}

// ── Main ───────────────────────────────────────────────────────────────────

async function run() {
	let browser;
	try {
		// ── Launch browser ───────────────────────────────────────────────────
		browser = await chromium.launch({ headless: true });
		const context = await browser.newContext({
			reducedMotion: "no-preference",
		});
		const page = await context.newPage();

		// ── Navigate to live signals ─────────────────────────────────────────
		await page.goto(`${DASHBOARD_URL}/signals`, {
			waitUntil: "domcontentloaded",
		});
		pass("navigate to /signals");

		// ── Assert replay rows rendered ──────────────────────────────────────
		const replayRowCount = await waitForRows(
			page,
			REPLAY_COUNT,
			REPLAY_WAIT_MS,
		);
		assert(
			replayRowCount >= REPLAY_COUNT,
			`replay rows rendered (${replayRowCount} >= ${REPLAY_COUNT})`,
		);

		// Check all replay rows have replay phase.
		const replaySignalIds = await collectSignalIds(page);
		const replayPhases = await collectSignalPhases(page);
		let replayOk = true;
		for (let i = 0; i < replaySignalIds.length; i++) {
			if (replayPhases[i] !== "replay") {
				fail(
					`replay phase on signal #${replaySignalIds[i]}`,
					`expected "replay", got "${replayPhases[i]}"`,
				);
				replayOk = false;
			}
		}
		if (replayOk && replaySignalIds.length >= REPLAY_COUNT) {
			pass('all replay rows have data-signal-phase="replay"');
		}

		// ── Wait for live signal ─────────────────────────────────────────────
		await page.waitForTimeout(LIVE_WAIT_MS);
		const allPhasesAfterLive = await collectSignalPhases(page);
		const allIdsAfterLive = await collectSignalIds(page);

		const liveRows = [];
		for (let i = 0; i < allPhasesAfterLive.length; i++) {
			if (allPhasesAfterLive[i] === "live") liveRows.push(allIdsAfterLive[i]);
		}

		assert(
			liveRows.length === 1,
			`exactly one live row (found ${liveRows.length})`,
		);
		if (liveRows.length === 1) {
			pass(`live row has id=${liveRows[0]} and data-signal-phase="live"`);
		}

		// ── Assert no duplicate signal ids in the DOM ────────────────────────
		const allIdSet = new Set(allIdsAfterLive);
		assert(
			allIdSet.size === allIdsAfterLive.length,
			`no duplicate signal ids (${allIdsAfterLive.length} rows, ${allIdSet.size} unique)`,
		);

		// ── Reconnect: force SSE disconnect via mock control endpoint ────────
		const disconnectRes = await new Promise((resolve, reject) => {
			const req = http.request(
				`${MOCK_LIVE_URL}/__control/disconnect`,
				{ method: "POST" },
				(res) => {
					let body = "";
					res.on("data", (d) => {
						body += d;
					});
					res.on("end", () => resolve({ status: res.statusCode, body }));
				},
			);
			req.on("error", reject);
			req.end();
		});

		assert(
			disconnectRes.status === 200,
			`mock disconnect trigger accepted (HTTP ${disconnectRes.status})`,
		);

		// Wait for the frontend to reconnect and settle.
		await page.waitForTimeout(5_000);

		// Collect signal ids after reconnect.
		const postReconnectIds = await collectSignalIds(page);
		const postReconnectIdSet = new Set(postReconnectIds);
		assert(
			postReconnectIdSet.size === postReconnectIds.length,
			`no duplicate signal ids after reconnect (${postReconnectIds.length} rows, ${postReconnectIdSet.size} unique)`,
		);

		// ── Reduced-motion run ───────────────────────────────────────────────
		await context.close();
		const reducedContext = await browser.newContext({
			reducedMotion: "reduce",
		});
		const reducedPage = await reducedContext.newPage();
		await reducedPage.goto(`${DASHBOARD_URL}/signals`, {
			waitUntil: "domcontentloaded",
		});

		await reducedPage.waitForTimeout(LIVE_WAIT_MS + REPLAY_WAIT_MS);

		const reducedLiveRows = await reducedPage
			.locator('[data-signal-phase="live"]')
			.count();
		assert(reducedLiveRows >= 1, "live row rendered in reduced-motion run");

		const liveMotionPath = await reducedPage
			.locator('[data-signal-phase="live"]')
			.first()
			.getAttribute("data-motion-path");
		assert(
			liveMotionPath === "reduced",
			`live row has data-motion-path="reduced" (got "${liveMotionPath}")`,
		);

		// Assert replay rows also have reduced motion path.
		const replayMotionPaths = await reducedPage
			.locator('[data-signal-phase="replay"]')
			.evaluateAll((els) =>
				els.map((el) => el.getAttribute("data-motion-path")),
			);

		let replayMotionOk = true;
		for (let i = 0; i < replayMotionPaths.length; i++) {
			if (replayMotionPaths[i] !== "reduced") {
				fail(
					`reduced-motion replay row #${i}`,
					`expected "reduced", got "${replayMotionPaths[i]}"`,
				);
				replayMotionOk = false;
			}
		}
		if (replayMotionOk && replayMotionPaths.length > 0) {
			pass(
				`all replay rows have data-motion-path="reduced" in reduced-motion run (${replayMotionPaths.length} rows)`,
			);
		}

		await reducedContext.close();
		pass("clean browser shutdown");

		// ── Summary ──────────────────────────────────────────────────────────
		summary();
		process.exit(0);
	} catch (err) {
		console.error("Playwright smoke failed:", err);
		process.exit(1);
	} finally {
		if (browser) await browser.close().catch(() => {});
	}
}

run();
