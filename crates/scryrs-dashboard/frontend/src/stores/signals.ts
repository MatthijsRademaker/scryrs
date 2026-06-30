import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
	getSignalStreamUrl,
	type DashboardSignal,
	type HotspotSignal,
} from "@/shared/api/client";

type ConnectionState =
	| "idle"
	| "connecting"
	| "connected"
	| "reconnecting"
	| "error";

/** A streamed signal plus whether it arrived live (vs. as replayed history). */
export interface FeedSignal extends DashboardSignal {
	/**
	 * `true` when the signal arrived after the stream caught up (a live tail
	 * signal that should ignite), `false` when it was part of the replayed
	 * history burst (which fades in calmly). Derived from stream timing, never
	 * from signal content.
	 */
	live: boolean;
}

const RECONNECT_DELAY_MS = 1_000;
// The server chains replayed history directly into the live broadcast on the
// same SSE stream with no boundary marker, so we infer the boundary: replayed
// signals arrive as a back-to-back burst, then the stream goes quiet before
// live signals trickle in. Once the stream has been quiet this long, we treat
// subsequent signals as live.
const REPLAY_SETTLE_MS = 350;

export const useSignalStore = defineStore("signals", () => {
	const signals = ref<FeedSignal[]>([]);
	const lastSeenId = ref(0);
	const connectionState = ref<ConnectionState>("idle");
	const error = ref<string | null>(null);

	let eventSource: EventSource | null = null;
	let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	let settleTimer: ReturnType<typeof setTimeout> | null = null;
	// "replay" until the stream has been quiet for REPLAY_SETTLE_MS, then "live".
	let phase: "replay" | "live" = "replay";

	// Newest-first ordering for the live feed.
	const feed = computed(() => signals.value.slice().reverse());

	function clearSettleTimer() {
		if (settleTimer !== null) {
			clearTimeout(settleTimer);
			settleTimer = null;
		}
	}

	// (Re)start the quiescence timer; when it fires, the catch-up burst is over
	// and the stream is considered live.
	function armSettleTimer() {
		clearSettleTimer();
		settleTimer = setTimeout(() => {
			settleTimer = null;
			phase = "live";
		}, REPLAY_SETTLE_MS);
	}

	function closeSource() {
		eventSource?.close();
		eventSource = null;
	}

	function clearReconnectTimer() {
		if (reconnectTimer !== null) {
			clearTimeout(reconnectTimer);
			reconnectTimer = null;
		}
	}

	function appendSignal(id: number, payload: HotspotSignal) {
		if (signals.value.some((signal) => signal.id === id)) {
			lastSeenId.value = Math.max(lastSeenId.value, id);
			return;
		}

		signals.value.push({ ...payload, id, live: phase === "live" });
		lastSeenId.value = Math.max(lastSeenId.value, id);

		// While still catching up, each replayed signal extends the quiet window;
		// the stream only becomes "live" once it falls silent.
		if (phase === "replay") {
			armSettleTimer();
		}
	}

	function parseSignalId(rawId: string): number {
		const id = Number(rawId);
		if (!Number.isInteger(id) || id < 0) {
			throw new Error(`invalid signal id: ${rawId}`);
		}
		return id;
	}

	function connect(after: number) {
		clearReconnectTimer();
		clearSettleTimer();
		closeSource();
		// Every (re)connect starts by catching up on persisted history.
		phase = "replay";
		connectionState.value =
			after === 0 && signals.value.length === 0 ? "connecting" : "reconnecting";

		const source = new EventSource(getSignalStreamUrl(after));
		eventSource = source;

		source.onopen = () => {
			if (eventSource !== source) return;
			error.value = null;
			connectionState.value = "connected";
			// If no history is replayed, the stream is live after the quiet window.
			armSettleTimer();
		};

		source.onmessage = (event) => {
			if (eventSource !== source) return;
			try {
				const id = parseSignalId(event.lastEventId);
				const payload = JSON.parse(event.data) as HotspotSignal;
				appendSignal(id, payload);
			} catch (unknownError) {
				error.value =
					unknownError instanceof Error
						? unknownError.message
						: "Signal payload could not be parsed";
				connectionState.value = "error";
				stop();
			}
		};

		source.onerror = () => {
			if (eventSource !== source) return;
			source.close();
			eventSource = null;
			clearSettleTimer();
			error.value = "Signal stream disconnected";
			connectionState.value = "reconnecting";
			reconnectTimer = setTimeout(() => {
				reconnectTimer = null;
				connect(lastSeenId.value);
			}, RECONNECT_DELAY_MS);
		};
	}

	function start() {
		if (eventSource || reconnectTimer) return;
		connect(lastSeenId.value);
	}

	function stop() {
		clearReconnectTimer();
		clearSettleTimer();
		closeSource();
		if (connectionState.value !== "error") {
			connectionState.value = "idle";
		}
	}

	return { signals, feed, lastSeenId, connectionState, error, start, stop };
});
