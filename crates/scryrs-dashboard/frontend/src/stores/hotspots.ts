import { defineStore } from "pinia";
import { computed, ref, shallowRef } from "vue";
import {
	getHotspots,
	type HotspotsReport,
	type HotspotEntry,
} from "@/shared/api/client";

export type PollState = "idle" | "polling" | "updating" | "stale" | "error";

export interface DeltaEntry extends HotspotEntry {
	/** Classification for this poll cycle: 'entered', 'changed', or 'unchanged'. */
	deltaType: "entered" | "changed" | "unchanged";
	/** True when this entry's score increased vs. the previous poll. */
	scoreIncreased: boolean;
	/** True when this entry's rank changed vs. the previous poll. */
	rankChanged: boolean;
}

export const useHotspotStore = defineStore("hotspots", () => {
	const report = ref<HotspotsReport | null>(null);
	const loading = ref(false);
	const error = ref<string | null>(null);
	const entries = computed(() => report.value?.entries ?? []);

	// ── Polling lifecycle ──────────────────────────────────────────────────
	const pollState = ref<PollState>("idle");
	const lastUpdated = ref<number | null>(null);
	const staleError = ref<string | null>(null);
	let pollTimer: ReturnType<typeof setInterval> | null = null;
	const lastGoodReport = shallowRef<HotspotsReport | null>(null);

	// ── Delta computation ──────────────────────────────────────────────────
	const prevEntries = ref<Map<string, HotspotEntry>>(new Map());
	const isFirstLoad = ref(true);
	const enteredKeys = ref<Set<string>>(new Set());
	const changedKeys = ref<Set<string>>(new Set());
	const exitedKeys = ref<Set<string>>(new Set());
	/** Per-entry flags computed during delta so entriesWithDelta has stable old values. */
	const scoreIncreasedMap = ref<Map<string, boolean>>(new Map());
	const rankChangedMap = ref<Map<string, boolean>>(new Map());

	function entryKey(
		entry: HotspotEntry | { subjectKind: string; subject: string },
	): string {
		return `${entry.subjectKind}:${entry.subject}`;
	}

	/** Entries decorated with delta classification + scoreIncreased/rankChanged flags. */
	const entriesWithDelta = computed<DeltaEntry[]>(() => {
		return entries.value.map((entry) => {
			const key = entryKey(entry);
			const deltaType: DeltaEntry["deltaType"] = isFirstLoad.value
				? "unchanged"
				: enteredKeys.value.has(key)
					? "entered"
					: changedKeys.value.has(key)
						? "changed"
						: "unchanged";
			const scoreIncreased = scoreIncreasedMap.value.get(key) ?? false;
			const rankChanged = rankChangedMap.value.get(key) ?? false;
			return { ...entry, deltaType, scoreIncreased, rankChanged };
		});
	});

	function computeDelta(newEntries: HotspotEntry[]) {
		if (isFirstLoad.value) {
			// First load — populate without flags.
			const map = new Map<string, HotspotEntry>();
			for (const entry of newEntries) {
				map.set(entryKey(entry), entry);
			}
			prevEntries.value = map;
			enteredKeys.value = new Set();
			changedKeys.value = new Set();
			exitedKeys.value = new Set();
			scoreIncreasedMap.value = new Map();
			rankChangedMap.value = new Map();
			return;
		}

		const oldMap = prevEntries.value;
		const newMap = new Map<string, HotspotEntry>();
		const entered = new Set<string>();
		const changed = new Set<string>();
		const exited = new Set(oldMap.keys());
		const scoreIncreased = new Map<string, boolean>();
		const rankChanged = new Map<string, boolean>();

		for (const entry of newEntries) {
			const key = entryKey(entry);
			newMap.set(key, entry);
			if (!oldMap.has(key)) {
				entered.add(key);
				exited.delete(key);
				scoreIncreased.set(key, false);
				rankChanged.set(key, false);
			} else {
				exited.delete(key);
				const old = oldMap.get(key)!;
				if (old.score !== entry.score || old.rank !== entry.rank) {
					changed.add(key);
					scoreIncreased.set(key, old.score < entry.score);
					rankChanged.set(key, old.rank !== entry.rank);
				} else {
					scoreIncreased.set(key, false);
					rankChanged.set(key, false);
				}
			}
		}

		prevEntries.value = newMap;
		enteredKeys.value = entered;
		changedKeys.value = changed;
		exitedKeys.value = exited;
		scoreIncreasedMap.value = scoreIncreased;
		rankChangedMap.value = rankChanged;
	}

	// ── Core load (shared by manual load() and poll ticks) ────────────────
	async function fetchAndUpdate(isPoll: boolean) {
		loading.value = true;
		if (isPoll) pollState.value = "updating";
		try {
			const fresh = await getHotspots();
			report.value = fresh;
			lastGoodReport.value = fresh;
			error.value = null;
			staleError.value = null;
			lastUpdated.value = Date.now();
			computeDelta(fresh.entries);
			isFirstLoad.value = false;
			if (isPoll) pollState.value = "polling";
		} catch (unknownError) {
			const message =
				unknownError instanceof Error
					? unknownError.message
					: "Hotspot data could not be read";
			if (isPoll) {
				// Preserve last good report on poll failure.
				staleError.value = message;
				pollState.value = "stale";
			} else {
				error.value = message;
				pollState.value = "error";
			}
		} finally {
			loading.value = false;
		}
	}

	// ── Manual load (local mode or initial fetch) ──────────────────────────
	async function load() {
		await fetchAndUpdate(false);
	}

	// ── Polling ────────────────────────────────────────────────────────────
	function pollTick() {
		if (loading.value) return; // In-flight guard.
		void fetchAndUpdate(true);
	}

	/** Start periodic polling against `/api/hotspots`. */
	function startPolling(intervalMs = 15_000) {
		if (pollTimer !== null) {
			clearInterval(pollTimer);
			pollTimer = null;
		}
		pollState.value = "polling";
		// Immediate first fetch.
		void fetchAndUpdate(true);
		pollTimer = setInterval(pollTick, intervalMs);
	}

	/** Stop polling and clear the interval timer. */
	function stopPolling() {
		if (pollTimer !== null) {
			clearInterval(pollTimer);
			pollTimer = null;
		}
		if (pollState.value !== "stale" && pollState.value !== "error") {
			pollState.value = "idle";
		}
	}

	// ── Tab-visibility pause/resume ────────────────────────────────────────
	let visibilityDebounce: ReturnType<typeof setTimeout> | null = null;
	let visibilityWired = false;

	function onVisibilityChange() {
		if (visibilityDebounce !== null) {
			clearTimeout(visibilityDebounce);
			visibilityDebounce = null;
		}
		visibilityDebounce = setTimeout(() => {
			visibilityDebounce = null;
			if (document.hidden) {
				// Pause: clear the timer but don't reset pollState.
				if (pollTimer !== null) {
					clearInterval(pollTimer);
					pollTimer = null;
				}
			} else {
				// Resume: if we were polling before pause, restart with immediate fetch.
				if (pollState.value === "polling" || pollState.value === "stale") {
					startPolling();
				}
			}
		}, 500);
	}

	function wireVisibility() {
		if (visibilityWired) return;
		visibilityWired = true;
		document.addEventListener("visibilitychange", onVisibilityChange);
	}

	function unwireVisibility() {
		visibilityWired = false;
		if (visibilityDebounce !== null) {
			clearTimeout(visibilityDebounce);
			visibilityDebounce = null;
		}
		document.removeEventListener("visibilitychange", onVisibilityChange);
	}

	/** Start polling (intended for live mode) with visibility listener. */
	function startPollingWithVisibility(intervalMs?: number) {
		startPolling(intervalMs);
		wireVisibility();
	}

	/** Stop polling and remove visibility listener. Full teardown for unmount. */
	function stopPollingFull() {
		stopPolling();
		unwireVisibility();
	}

	return {
		report,
		entries,
		loading,
		error,
		load,
		// Polling lifecycle
		pollState,
		lastUpdated,
		staleError,
		startPolling: startPollingWithVisibility,
		stopPolling: stopPollingFull,
		startPollingRaw: startPolling,
		stopPollingRaw: stopPolling,
		// Delta
		entriesWithDelta,
		enteredKeys,
		changedKeys,
		exitedKeys,
		isFirstLoad,
		prevEntries,
	};
});
