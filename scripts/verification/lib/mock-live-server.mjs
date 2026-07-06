/**
 * Deterministic mock SSE server for browser-level live-signals verification.
 *
 * Protocol:
 *   GET /v1/repositories/{repository_id}/signals?after=N
 *     - Content-Type: text/event-stream
 *     - id: <N> + data: <JSON> + blank line
 *   GET /v1/repositories/{repository_id}/hotspots
 *     - Content-Type: application/json
 *     - Minimal valid LiveHotspotsResponse
 *   POST /__control/disconnect
 *     - Marks the next SSE write to terminate the stream (simulates disconnect)
 *
 * Signal ids are predefined so Playwright tests can assert known identities.
 * The mock emits replay signals (ids 1..REPLAY_COUNT) back-to-back with
 * <100ms inter-signal gaps, then falls silent for >=500ms, then emits one
 * live signal (id REPLAY_COUNT+1).
 */
import http from "node:http";
import fs from "node:fs";

// ── Configuration ──────────────────────────────────────────────────────────

const PORT = parseInt(process.env.MOCK_LIVE_PORT || "0", 10);
const REPLAY_COUNT = parseInt(process.env.REPLAY_COUNT || "10", 10);
const INTER_SIGNAL_MS = 50; // <100ms per spec
const SETTLE_WINDOW_MS = 1_000; // Above REPLAY_SETTLE_MS=350 with proxy buffering margin

// Write a pidfile so the orchestration script knows where the mock server is.
const PIDFILE = process.env.MOCK_LIVE_PIDFILE || "";

// ── Signal payloads ────────────────────────────────────────────────────────

/**
 * Build a signal event payload for a given id.
 * Replay signals: ids 1..REPLAY_COUNT
 * Live signal: id REPLAY_COUNT + 1
 */
function makeSignal(id) {
	const isLive = id > REPLAY_COUNT;
	return {
		repositoryId: "repo-a",
		subjectKind: "file",
		subject: `src/module-${String(id).padStart(2, "0")}.rs`,
		score: 50 + id * 5,
		delta: isLive ? 8 : 2,
		window: "cumulative",
		threshold: 10,
		evidenceRowIds: [id * 10 + 1],
		createdAt: new Date(Date.UTC(2026, 6, 5, 0, 0, id)).toISOString(),
	};
}

// ── SSE helpers ────────────────────────────────────────────────────────────

function sseEvent(res, id, data) {
	res.write(`id: ${id}\n`);
	res.write(`data: ${JSON.stringify(data)}\n\n`);
}

// ── Request router ─────────────────────────────────────────────────────────

const server = http.createServer((req, res) => {
	const url = new URL(req.url, `http://${req.headers.host || "localhost"}`);

	// ── Hotspots stub ──────────────────────────────────────────────────────
	if (
		req.method === "GET" &&
		url.pathname.match(/^\/v1\/repositories\/[^/]+\/hotspots$/)
	) {
		const repoId = url.pathname.split("/")[3];
		res.writeHead(200, { "Content-Type": "application/json" });
		res.end(
			JSON.stringify({
				schemaVersion: "1.0",
				repositoryId: repoId,
				cursor: "0",
				generatedAt: new Date().toISOString(),
				entries: [],
			}),
		);
		return;
	}

	// ── Control: scripted disconnect ───────────────────────────────────────
	if (req.method === "POST" && url.pathname === "/__control/disconnect") {
		// Terminate the active SSE connection if one exists.
		if (server._activeSSE) {
			server._activeSSE.end();
			server._activeSSE = null;
		}
		res.writeHead(200, { "Content-Type": "text/plain" });
		res.end("ok");
		return;
	}

	// ── Signals SSE endpoint ───────────────────────────────────────────────
	if (
		req.method === "GET" &&
		url.pathname.match(/^\/v1\/repositories\/[^/]+\/signals$/)
	) {
		const afterParam = parseInt(url.searchParams.get("after") || "0", 10);

		res.writeHead(200, {
			"Content-Type": "text/event-stream",
			"Cache-Control": "no-cache",
			Connection: "keep-alive",
			"X-Accel-Buffering": "no",
		});

		// Disable Nagle's algorithm so each SSE event is sent immediately
		// as a TCP segment, preserving inter-signal timing for the frontend.
		if (req.socket) {
			req.socket.setNoDelay(true);
		}

		// Track the active SSE response for scripted disconnect.
		server._activeSSE = res;
		req.on("close", () => {
			if (server._activeSSE === res) {
				server._activeSSE = null;
			}
		});

		// Determine the start id: on reconnect, start after the cursor.
		const startId = Math.max(1, afterParam + 1);
		let nextId = startId;

		function emitNext() {
			const totalSignals = REPLAY_COUNT + 1;

			if (nextId > totalSignals) {
				return;
			}

			// Diagnostic: log when each signal is written.
			const ts = Date.now();
			const isReplay = nextId <= REPLAY_COUNT;
			console.error(
				`[mock] ts=${ts} id=${nextId} phase=${isReplay ? "replay" : "live"}`,
			);

			const signal = makeSignal(nextId);
			sseEvent(res, nextId, signal);
			nextId++;

			// Determine the delay before the next signal based on the new nextId.
			if (nextId <= REPLAY_COUNT) {
				// Still more replay signals to emit, use short inter-signal delay.
				setTimeout(emitNext, INTER_SIGNAL_MS);
			} else if (nextId === REPLAY_COUNT + 1) {
				// Just finished the replay burst; wait for the settle window before live.
				setTimeout(emitNext, SETTLE_WINDOW_MS);
			}
			// else: live signal already emitted, keep connection open silently.
		}

		emitNext();

		// Clean up on client disconnect.
		req.on("close", () => {
			res.end();
		});
		return;
	}

	// ── Unknown route ──────────────────────────────────────────────────────
	res.writeHead(404, { "Content-Type": "text/plain" });
	res.end("not found");
});

// ── Startup ────────────────────────────────────────────────────────────────

server.listen(PORT, "127.0.0.1", () => {
	const addr = server.address();
	const port = addr.port;
	console.log(`MOCK_LIVE_SERVER=http://127.0.0.1:${port}`);

	// Write pidfile if requested.
	if (PIDFILE) {
		fs.writeFileSync(PIDFILE, String(process.pid));
	}
});
