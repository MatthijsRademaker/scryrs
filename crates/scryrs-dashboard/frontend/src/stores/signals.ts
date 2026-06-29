import { defineStore } from "pinia";
import { ref } from "vue";
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

const RECONNECT_DELAY_MS = 1_000;

export const useSignalStore = defineStore("signals", () => {
	const signals = ref<DashboardSignal[]>([]);
	const lastSeenId = ref(0);
	const connectionState = ref<ConnectionState>("idle");
	const error = ref<string | null>(null);

	let eventSource: EventSource | null = null;
	let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

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

		signals.value.push({ id, ...payload });
		lastSeenId.value = Math.max(lastSeenId.value, id);
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
		closeSource();
		connectionState.value =
			after === 0 && signals.value.length === 0 ? "connecting" : "reconnecting";

		const source = new EventSource(getSignalStreamUrl(after));
		eventSource = source;

		source.onopen = () => {
			if (eventSource !== source) return;
			error.value = null;
			connectionState.value = "connected";
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
		closeSource();
		if (connectionState.value !== "error") {
			connectionState.value = "idle";
		}
	}

	return { signals, lastSeenId, connectionState, error, start, stop };
});
