#!/usr/bin/env node

/**
 * scryrs live-hotspots end-to-end verification fixture.
 *
 * Proves the shipped `scryrs` binary can:
 *   - start `scryrs server` headlessly with a fresh server-owned SQLite store
 *   - accept remote `scryrs record --file` submissions from two agents
 *   - accumulate one shared hotspot across multiple sessions/agents
 *   - acknowledge duplicate replay idempotently without changing state
 *   - replay and resume `HotspotSignal` SSE streams with `after=` cursors
 *
 * Prerequisites:
 *   - Real `scryrs` binary (SCRYRS_BIN env, or target/release/scryrs)
 *
 * Usage: node scripts/verification/live-hotspots-e2e.mjs
 */

import { mkdtempSync, rmSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { createServer } from "node:net";
import http from "node:http";
import { fileURLToPath } from "node:url";

import { pass, fail, summary } from "./lib/assert.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = join(__dirname, "..", "..");
const SCRYRS_BIN =
	process.env.SCRYRS_BIN || join(ROOT, "target", "release", "scryrs");

const REPOSITORY_ID = "repo-live-hotspots";
const WORKSPACE_ID = "workspace-live-hotspots";
const SUBJECT_PATH = "src/auth.ts";
const PROBE_REPOSITORY_ID = "probe-live-hotspots";
const FIXTURE_TIMEOUT_MS = 120_000;
const SERVER_READY_TIMEOUT_MS = 15_000;
const REMOTE_TIMEOUT_MS = "10000";
const SSE_IDLE_MS = 750;
const SSE_OVERALL_MS = 5_000;

function freshDir(prefix) {
	return mkdtempSync(join(tmpdir(), prefix));
}

function nowIso(offsetMinutes) {
	const base = Date.parse("2026-06-29T12:00:00Z");
	return new Date(base + offsetMinutes * 60_000)
		.toISOString()
		.replace(".000Z", "Z");
}

function makeEditMadeLine(sessionId, target, timestamp) {
	return JSON.stringify({
		schema_version: "0.1.0",
		timestamp,
		session_id: sessionId,
		event_type: "EditMade",
		tool_name: "edit",
		payload: {
			type: "EditMade",
			target,
		},
		outcome: { result: "Success" },
	});
}

function writeJsonl(path, lines) {
	writeFileSync(path, `${lines.join("\n")}\n`);
}

function runScryrs(args, { cwd, env } = {}) {
	const result = spawnSync(SCRYRS_BIN, args, {
		cwd: cwd || ROOT,
		env: { ...process.env, ...env },
		encoding: "utf-8",
		timeout: 30_000,
	});
	return {
		status: result.status,
		stdout: result.stdout ?? "",
		stderr: result.stderr ?? "",
	};
}

function must(condition, name, reason) {
	if (!condition) {
		fail(name, reason);
		throw new Error(`${name}: ${reason}`);
	}
	pass(name);
}

function parseJson(name, text) {
	try {
		return JSON.parse(text);
	} catch (error) {
		throw new Error(`${name}: malformed JSON: ${error}\n${text}`);
	}
}

async function allocatePort() {
	return await new Promise((resolve, reject) => {
		const server = createServer();
		server.on("error", reject);
		server.listen(0, "127.0.0.1", () => {
			const address = server.address();
			if (!address || typeof address === "string") {
				server.close(() => reject(new Error("failed to allocate TCP port")));
				return;
			}
			const port = address.port;
			server.close((error) => {
				if (error) reject(error);
				else resolve(port);
			});
		});
	});
}

function sleep(ms) {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function getText(url) {
	return new Promise((resolve, reject) => {
		const req = http.get(url, (res) => {
			let body = "";
			res.setEncoding("utf8");
			res.on("data", (chunk) => {
				body += chunk;
			});
			res.on("end", () => {
				resolve({ statusCode: res.statusCode ?? 0, body });
			});
		});
		req.on("error", reject);
	});
}

async function waitForServerReady(baseUrl) {
	const deadline = Date.now() + SERVER_READY_TIMEOUT_MS;
	const url = `${baseUrl}/v1/repositories/${PROBE_REPOSITORY_ID}/hotspots?window=cumulative`;
	let lastError = "not started";

	while (Date.now() < deadline) {
		try {
			const { statusCode, body } = await getText(url);
			if (statusCode === 200) {
				parseJson("server readiness response", body);
				return;
			}
			lastError = `unexpected HTTP ${statusCode}: ${body}`;
		} catch (error) {
			lastError = error instanceof Error ? error.message : String(error);
		}
		await sleep(200);
	}

	throw new Error(
		`server readiness timeout after ${SERVER_READY_TIMEOUT_MS}ms: ${lastError}`,
	);
}

async function startServer(storePath) {
	let lastError = "server did not start";

	for (let attempt = 1; attempt <= 3; attempt += 1) {
		const port = await allocatePort();
		const stderr = [];
		const child = spawn(
			SCRYRS_BIN,
			[
				"server",
				"--bind",
				"127.0.0.1",
				"--port",
				String(port),
				"--store",
				storePath,
			],
			{
				cwd: ROOT,
				env: { ...process.env },
				stdio: ["ignore", "pipe", "pipe"],
			},
		);

		child.stdout.setEncoding("utf8");
		child.stderr.setEncoding("utf8");
		child.stderr.on("data", (chunk) => stderr.push(chunk));

		try {
			const baseUrl = `http://127.0.0.1:${port}`;
			await waitForServerReady(baseUrl);
			return { child, port, baseUrl, stderr };
		} catch (error) {
			lastError = `${error instanceof Error ? error.message : String(error)}\n${stderr.join("")}`;
			child.kill("SIGTERM");
			await waitForExit(child, 5_000).catch(() => undefined);
		}
	}

	throw new Error(`failed to start scryrs server: ${lastError}`);
}

async function waitForExit(child, timeoutMs) {
	if (child.exitCode !== null) {
		return child.exitCode;
	}
	return await new Promise((resolve, reject) => {
		const timer = setTimeout(
			() => reject(new Error(`process exit timeout after ${timeoutMs}ms`)),
			timeoutMs,
		);
		child.once("exit", (code) => {
			clearTimeout(timer);
			resolve(code ?? 0);
		});
		child.once("error", (error) => {
			clearTimeout(timer);
			reject(error);
		});
	});
}

async function stopServer(server) {
	if (!server) return;
	const { child } = server;
	if (child.exitCode !== null) return;
	child.kill("SIGTERM");
	try {
		await waitForExit(child, 5_000);
	} catch {
		child.kill("SIGKILL");
		await waitForExit(child, 5_000).catch(() => undefined);
	}
}

async function getHotspots(baseUrl) {
	const url = `${baseUrl}/v1/repositories/${REPOSITORY_ID}/hotspots?window=cumulative`;
	const { statusCode, body } = await getText(url);
	if (statusCode !== 200) {
		throw new Error(`hotspot query failed with HTTP ${statusCode}: ${body}`);
	}
	return parseJson("hotspot query", body);
}

function findSubjectEntry(doc) {
	return (doc.entries || []).find(
		(entry) => entry.subjectKind === "file" && entry.subject === SUBJECT_PATH,
	);
}

function projectEntry(entry) {
	return {
		rank: entry.rank,
		subjectKind: entry.subjectKind,
		subject: entry.subject,
		score: entry.score,
		counts: entry.counts,
		sessionCount: entry.sessionCount,
		firstSeen: entry.firstSeen,
		lastSeen: entry.lastSeen,
		evidence: entry.evidence,
	};
}

async function collectSseEvents(baseUrl, after) {
	const url = new URL(`${baseUrl}/v1/repositories/${REPOSITORY_ID}/signals`);
	url.searchParams.set("after", String(after));

	return await new Promise((resolve, reject) => {
		const events = [];
		let settled = false;
		let endedByTimeout = false;
		let buffer = "";
		let currentId = null;
		let currentData = [];

		const finish = (callback) => {
			if (settled) return;
			settled = true;
			clearTimeout(idleTimer);
			clearTimeout(overallTimer);
			callback();
		};

		const flushEvent = () => {
			if (currentId === null && currentData.length === 0) {
				return;
			}
			if (currentId !== null && currentData.length > 0) {
				events.push({
					id: Number(currentId),
					data: parseJson("SSE data", currentData.join("\n")),
				});
			}
			currentId = null;
			currentData = [];
		};

		const resetIdle = () => {
			clearTimeout(idleTimer);
			idleTimer = setTimeout(() => {
				endedByTimeout = true;
				req.destroy();
			}, SSE_IDLE_MS);
		};

		let idleTimer = setTimeout(() => {
			endedByTimeout = true;
			req.destroy();
		}, SSE_IDLE_MS);
		const overallTimer = setTimeout(() => {
			finish(() =>
				reject(
					new Error(`SSE overall timeout after ${SSE_OVERALL_MS}ms for ${url}`),
				),
			);
			req.destroy();
		}, SSE_OVERALL_MS);

		const req = http.get(url, (res) => {
			if ((res.statusCode ?? 0) !== 200) {
				finish(() => reject(new Error(`SSE HTTP ${res.statusCode}: ${url}`)));
				res.resume();
				return;
			}
			res.setEncoding("utf8");
			resetIdle();
			res.on("data", (chunk) => {
				resetIdle();
				buffer += chunk;
				let newlineIndex = buffer.indexOf("\n");
				while (newlineIndex !== -1) {
					const line = buffer.slice(0, newlineIndex).replace(/\r$/, "");
					buffer = buffer.slice(newlineIndex + 1);
					if (line === "") {
						flushEvent();
					} else if (line.startsWith(":")) {
						// keep-alive/comment
					} else if (line.startsWith("id:")) {
						currentId = line.slice(3).trim();
					} else if (line.startsWith("data:")) {
						currentData.push(line.slice(5).trimStart());
					}
					newlineIndex = buffer.indexOf("\n");
				}
			});
			res.on("end", () => {
				flushEvent();
				finish(() => resolve(events));
			});
			res.on("close", () => {
				flushEvent();
				if (endedByTimeout) {
					finish(() => resolve(events));
				}
			});
		});

		req.on("error", (error) => {
			flushEvent();
			if (endedByTimeout) {
				finish(() => resolve(events));
				return;
			}
			finish(() => reject(error));
		});
	});
}

function makeRecordEnv(baseUrl, agentId) {
	return {
		SCRYRS_REMOTE_INGEST_URL: baseUrl,
		SCRYRS_REPOSITORY_ID: REPOSITORY_ID,
		SCRYRS_WORKSPACE_ID: WORKSPACE_ID,
		SCRYRS_AGENT_ID: agentId,
		SCRYRS_REMOTE_TIMEOUT_MS: REMOTE_TIMEOUT_MS,
	};
}

function runRemoteRecord(repoDir, filePath, baseUrl, agentId) {
	const result = runScryrs(["record", "--file", filePath], {
		cwd: repoDir,
		env: makeRecordEnv(baseUrl, agentId),
	});
	const stdoutJson = result.stdout.trim()
		? parseJson("remote record stdout", result.stdout)
		: null;
	return {
		...result,
		json: stdoutJson,
	};
}

async function main() {
	const fixtureDir = freshDir("scryrs-live-hotspots-");
	const repoDir = join(fixtureDir, "repo");
	const dataDir = join(fixtureDir, "data");
	mkdirSync(repoDir, { recursive: true });
	mkdirSync(dataDir, { recursive: true });

	const agentAPath = join(dataDir, "agent-a.jsonl");
	const agentBPath = join(dataDir, "agent-b.jsonl");
	const storePath = join(dataDir, "server.db");

	writeJsonl(agentAPath, [
		makeEditMadeLine("session-agent-a", SUBJECT_PATH, nowIso(0)),
		makeEditMadeLine("session-agent-a", SUBJECT_PATH, nowIso(1)),
	]);
	writeJsonl(agentBPath, [
		makeEditMadeLine("session-agent-b", SUBJECT_PATH, nowIso(2)),
		makeEditMadeLine("session-agent-b", SUBJECT_PATH, nowIso(3)),
	]);

	let server;
	const fixtureTimer = setTimeout(() => {
		fail("fixture timeout", `exceeded ${FIXTURE_TIMEOUT_MS}ms`);
		summary();
	}, FIXTURE_TIMEOUT_MS);

	try {
		console.log(
			"\n\x1b[33m--- Live hotspots: headless end-to-end verification ---\x1b[0m",
		);

		must(
			runScryrs(["--version"]).status === 0,
			"scryrs binary is runnable",
			SCRYRS_BIN,
		);

		server = await startServer(storePath);
		pass(`server ready on ${server.baseUrl}`);

		const firstSubmission = runRemoteRecord(
			repoDir,
			agentAPath,
			server.baseUrl,
			"agent-a",
		);
		must(
			firstSubmission.status === 0,
			"agent A remote record exits 0",
			firstSubmission.stderr || `status=${firstSubmission.status}`,
		);
		must(
			firstSubmission.json?.transport === "remote",
			"agent A uses remote transport",
			JSON.stringify(firstSubmission.json),
		);
		must(
			firstSubmission.json?.accepted === 2,
			"agent A accepted count is 2",
			JSON.stringify(firstSubmission.json),
		);
		must(
			firstSubmission.json?.duplicate === 0,
			"agent A duplicate count is 0",
			JSON.stringify(firstSubmission.json),
		);
		must(
			firstSubmission.json?.rejected === 0,
			"agent A rejected count is 0",
			JSON.stringify(firstSubmission.json),
		);
		must(
			firstSubmission.json?.failed === 0,
			"agent A failed count is 0",
			JSON.stringify(firstSubmission.json),
		);

		const afterFirst = await getHotspots(server.baseUrl);
		const firstEntry = findSubjectEntry(afterFirst);
		must(
			Boolean(firstEntry),
			"hotspot exists after first submission",
			JSON.stringify(afterFirst),
		);
		must(
			firstEntry.score < 10,
			"first submission stays below threshold",
			JSON.stringify(firstEntry),
		);
		must(
			firstEntry.sessionCount === 1,
			"first submission sessionCount is 1",
			JSON.stringify(firstEntry),
		);
		must(
			firstEntry.evidence.rowIds.length === 2,
			"first submission evidence row count is 2",
			JSON.stringify(firstEntry.evidence),
		);

		const secondSubmission = runRemoteRecord(
			repoDir,
			agentBPath,
			server.baseUrl,
			"agent-b",
		);
		must(
			secondSubmission.status === 0,
			"agent B remote record exits 0",
			secondSubmission.stderr || `status=${secondSubmission.status}`,
		);
		must(
			secondSubmission.json?.accepted === 2,
			"agent B accepted count is 2",
			JSON.stringify(secondSubmission.json),
		);
		must(
			secondSubmission.json?.duplicate === 0,
			"agent B duplicate count is 0",
			JSON.stringify(secondSubmission.json),
		);
		must(
			secondSubmission.json?.rejected === 0,
			"agent B rejected count is 0",
			JSON.stringify(secondSubmission.json),
		);
		must(
			secondSubmission.json?.failed === 0,
			"agent B failed count is 0",
			JSON.stringify(secondSubmission.json),
		);

		const afterSecond = await getHotspots(server.baseUrl);
		const secondEntry = findSubjectEntry(afterSecond);
		must(
			Boolean(secondEntry),
			"hotspot exists after second submission",
			JSON.stringify(afterSecond),
		);
		must(
			secondEntry.score >= 10,
			"shared hotspot crossed threshold",
			JSON.stringify(secondEntry),
		);
		must(
			secondEntry.sessionCount === 2,
			"shared hotspot sessionCount is 2",
			JSON.stringify(secondEntry),
		);
		must(
			secondEntry.evidence.rowIds.length === 4,
			"shared hotspot evidence row count is 4",
			JSON.stringify(secondEntry.evidence),
		);
		must(
			JSON.stringify(secondEntry.evidence.rowIds.slice(0, 2)) ===
				JSON.stringify(firstEntry.evidence.rowIds),
			"second submission preserves first evidence row IDs as prefix",
			JSON.stringify({
				before: firstEntry.evidence.rowIds,
				after: secondEntry.evidence.rowIds,
			}),
		);
		must(
			secondEntry.counts?.eventType?.EditMade === 4,
			"shared hotspot EditMade count is 4",
			JSON.stringify(secondEntry.counts),
		);
		const projectedBeforeReplay = projectEntry(secondEntry);

		const signalsBeforeReplay = await collectSseEvents(server.baseUrl, 0);
		must(
			signalsBeforeReplay.length >= 1,
			"SSE after=0 replays at least one persisted signal",
			JSON.stringify(signalsBeforeReplay),
		);
		must(
			signalsBeforeReplay.every(
				(signal, index, list) => index === 0 || list[index - 1].id < signal.id,
			),
			"SSE replay order is ascending by signal id",
			JSON.stringify(signalsBeforeReplay.map((signal) => signal.id)),
		);
		const lastSeenId = signalsBeforeReplay.at(-1).id;
		const thresholdSignal = signalsBeforeReplay.find(
			(signal) =>
				signal.data.subjectKind === "file" &&
				signal.data.subject === SUBJECT_PATH,
		);
		must(
			Boolean(thresholdSignal),
			"threshold signal for shared hotspot is present",
			JSON.stringify(signalsBeforeReplay),
		);
		must(
			thresholdSignal.data.threshold === 10,
			"threshold signal uses default threshold 10",
			JSON.stringify(thresholdSignal.data),
		);
		must(
			thresholdSignal.data.window === "cumulative",
			"threshold signal window is cumulative",
			JSON.stringify(thresholdSignal.data),
		);
		must(
			JSON.stringify(thresholdSignal.data.evidenceRowIds) ===
				JSON.stringify(secondEntry.evidence.rowIds),
			"threshold signal evidence row IDs match cumulative hotspot evidence",
			JSON.stringify({
				signal: thresholdSignal.data.evidenceRowIds,
				hotspot: secondEntry.evidence.rowIds,
			}),
		);

		const replaySubmission = runRemoteRecord(
			repoDir,
			agentAPath,
			server.baseUrl,
			"agent-a",
		);
		must(
			replaySubmission.status === 0,
			"duplicate replay exits 0",
			replaySubmission.stderr || `status=${replaySubmission.status}`,
		);
		must(
			replaySubmission.json?.accepted === 0,
			"duplicate replay accepted count is 0",
			JSON.stringify(replaySubmission.json),
		);
		must(
			replaySubmission.json?.duplicate === 2,
			"duplicate replay duplicate count is 2",
			JSON.stringify(replaySubmission.json),
		);
		must(
			replaySubmission.json?.rejected === 0,
			"duplicate replay rejected count is 0",
			JSON.stringify(replaySubmission.json),
		);
		must(
			replaySubmission.json?.failed === 0,
			"duplicate replay failed count is 0",
			JSON.stringify(replaySubmission.json),
		);

		const afterReplay = await getHotspots(server.baseUrl);
		const replayEntry = findSubjectEntry(afterReplay);
		must(
			Boolean(replayEntry),
			"hotspot still exists after duplicate replay",
			JSON.stringify(afterReplay),
		);
		must(
			JSON.stringify(projectedBeforeReplay) ===
				JSON.stringify(projectEntry(replayEntry)),
			"duplicate replay does not change cumulative hotspot state",
			JSON.stringify({
				before: projectedBeforeReplay,
				after: projectEntry(replayEntry),
			}),
		);

		const signalsAfterReplay = await collectSseEvents(server.baseUrl, 0);
		must(
			JSON.stringify(signalsBeforeReplay) ===
				JSON.stringify(signalsAfterReplay),
			"duplicate replay does not change persisted signal history",
			JSON.stringify({
				before: signalsBeforeReplay,
				after: signalsAfterReplay,
			}),
		);

		const resumeSignals = await collectSseEvents(server.baseUrl, lastSeenId);
		must(
			resumeSignals.every((signal) => signal.id > lastSeenId),
			"SSE resume after=last_seen_id never replays previously seen ids",
			JSON.stringify({ lastSeenId, resumeSignals }),
		);
	} finally {
		clearTimeout(fixtureTimer);
		await stopServer(server);
		rmSync(fixtureDir, { recursive: true, force: true });
		summary();
	}
}

try {
	await main();
} catch (error) {
	fail(
		"live-hotspots fixture",
		error instanceof Error ? error.stack || error.message : String(error),
	);
	summary();
}
