import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useHotspotStore } from "@/stores/hotspots";
import * as client from "@/shared/api/client";
import type { HotspotsReport } from "@/shared/api/client";

function makeEntry(
	overrides: Partial<HotspotsReport["entries"][number]> = {},
): HotspotsReport["entries"][number] {
	return {
		rank: 1,
		subjectKind: "file",
		subject: "src/main.rs",
		score: 100,
		counts: {
			eventType: { commit: 5, merge: 2 },
			outcome: { success: 6, failure: 1 },
		},
		sessionCount: 3,
		firstSeen: "2026-06-01T00:00:00Z",
		lastSeen: "2026-06-29T00:00:00Z",
		evidence: { rowIds: [1, 2, 3] },
		...overrides,
	};
}

function makeReport(entries: HotspotsReport["entries"]): HotspotsReport {
	return {
		schemaVersion: "1",
		command: "hotspots",
		generatedAt: "2026-06-29T00:00:00Z",
		repositoryPath: "/repo",
		repositoryId: "repo-a",
		entries,
	};
}

describe("useHotspotStore", () => {
	beforeEach(() => {
		vi.useFakeTimers();
		setActivePinia(createPinia());
	});

	afterEach(() => {
		vi.restoreAllMocks();
		vi.useRealTimers();
	});

	// ── 6.2 Poll lifecycle ─────────────────────────────────────────────────
	describe("poll lifecycle", () => {
		it("startPollingRaw creates interval and fetches, stopPollingRaw clears it", async () => {
			const mock = vi
				.spyOn(client, "getHotspots")
				.mockResolvedValue(makeReport([makeEntry()]));
			const store = useHotspotStore();

			store.startPollingRaw(5_000);
			// Immediate first fetch is in-flight — state transitions to "updating".
			expect(store.pollState).toBe("updating");
			expect(mock).toHaveBeenCalledTimes(1);

			// Let the first fetch settle. State should return to "polling".
			await vi.advanceTimersByTimeAsync(0);
			expect(store.pollState).toBe("polling");
			expect(mock).toHaveBeenCalledTimes(1);

			// Advance past the interval — next tick fires.
			await vi.advanceTimersByTimeAsync(5_000);
			expect(mock).toHaveBeenCalledTimes(2);
			await vi.advanceTimersByTimeAsync(0);
			expect(store.pollState).toBe("polling");

			store.stopPollingRaw();
			// After stop, another interval tick should NOT fire.
			await vi.advanceTimersByTimeAsync(10_000);
			expect(mock).toHaveBeenCalledTimes(2);
			expect(store.pollState).toBe("idle");
		});

		it("stopPollingRaw preserves stale/error state", async () => {
			vi.spyOn(client, "getHotspots").mockRejectedValue(
				new Error("fetch failed"),
			);
			const store = useHotspotStore();

			store.startPollingRaw(5_000);
			// Immediate first fetch fails → state becomes "stale".
			await vi.advanceTimersByTimeAsync(0);
			expect(store.pollState).toBe("stale");

			store.stopPollingRaw();
			expect(store.pollState).toBe("stale"); // Preserves stale
		});
	});

	// ── 6.3 In-flight guard ────────────────────────────────────────────────
	describe("in-flight guard", () => {
		it("skips tick when loading is true", async () => {
			// Never-resolve promise so loading stays true.
			let resolve: (value: HotspotsReport) => void = () => {};
			const pending = new Promise<HotspotsReport>((r) => {
				resolve = r;
			});
			const mock = vi.spyOn(client, "getHotspots").mockReturnValue(pending);
			const store = useHotspotStore();

			store.startPollingRaw(1_000);
			// First immediate fetch is in-flight.
			expect(mock).toHaveBeenCalledTimes(1);

			// Advance past interval — tick should be skipped because loading is true.
			await vi.advanceTimersByTimeAsync(1_000);
			expect(mock).toHaveBeenCalledTimes(1);

			// Resolve the first fetch.
			resolve(makeReport([makeEntry()]));
			await vi.advanceTimersByTimeAsync(0);

			// Now advance — next tick should fire normally.
			await vi.advanceTimersByTimeAsync(1_000);
			expect(mock).toHaveBeenCalledTimes(2);

			store.stopPollingRaw();
		});
	});

	// ── 6.4 Error preservation ─────────────────────────────────────────────
	describe("error preservation", () => {
		it("preserves last good report on poll failure", async () => {
			const mock = vi.spyOn(client, "getHotspots");
			mock.mockResolvedValueOnce(
				makeReport([makeEntry({ subject: "a.rs", score: 10 })]),
			);
			mock.mockRejectedValueOnce(new Error("network down"));
			const store = useHotspotStore();

			store.startPollingRaw(5_000);
			// First fetch succeeds.
			await vi.advanceTimersByTimeAsync(0);
			expect(store.entries).toHaveLength(1);
			expect(store.entries[0].subject).toBe("a.rs");
			expect(store.staleError).toBeNull();
			expect(store.pollState).toBe("polling");

			// Second fetch fails.
			await vi.advanceTimersByTimeAsync(5_000);
			await vi.advanceTimersByTimeAsync(0);
			// Entries still accessible from last good report.
			expect(store.entries).toHaveLength(1);
			expect(store.entries[0].subject).toBe("a.rs");
			expect(store.staleError).toBe("network down");
			expect(store.pollState).toBe("stale");

			store.stopPollingRaw();
		});

		it("error field set on initial load failure with no cached data", async () => {
			vi.spyOn(client, "getHotspots").mockRejectedValue(new Error("init fail"));
			const store = useHotspotStore();

			await store.load();
			expect(store.error).toBe("init fail");
			expect(store.staleError).toBeNull();
			expect(store.entries).toHaveLength(0);
		});
	});

	// ── 6.5 Delta computation ──────────────────────────────────────────────
	describe("delta computation", () => {
		it("classifies entered, exited, and changed entries across polls", async () => {
			const mock = vi.spyOn(client, "getHotspots");
			// First poll: A(rank 1, score 100), B(rank 2, score 80)
			mock.mockResolvedValueOnce(
				makeReport([
					makeEntry({ subject: "a.rs", rank: 1, score: 100 }),
					makeEntry({ subject: "b.rs", rank: 2, score: 80 }),
				]),
			);
			// Second poll: B(rank 1, score 85), A(rank 2, score 100), C(rank 3, score 50)
			mock.mockResolvedValueOnce(
				makeReport([
					makeEntry({ subject: "b.rs", rank: 1, score: 85 }),
					makeEntry({ subject: "a.rs", rank: 2, score: 100 }),
					makeEntry({ subject: "c.rs", rank: 3, score: 50 }),
				]),
			);
			const store = useHotspotStore();

			store.startPollingRaw(5_000);
			await vi.advanceTimersByTimeAsync(0);
			// After first poll, entries are all unchanged.
			const firstDelta = store.entriesWithDelta;
			expect(firstDelta.every((e) => e.deltaType === "unchanged")).toBe(true);

			// Second poll.
			await vi.advanceTimersByTimeAsync(5_000);
			await vi.advanceTimersByTimeAsync(0);

			const delta = store.entriesWithDelta;
			const bySubject = (s: string) => delta.find((e) => e.subject === s)!;

			// C is entered.
			expect(bySubject("c.rs").deltaType).toBe("entered");
			// B is changed (rank and score both changed).
			expect(bySubject("b.rs").deltaType).toBe("changed");
			expect(bySubject("b.rs").scoreIncreased).toBe(true);
			expect(bySubject("b.rs").rankChanged).toBe(true);
			// A is changed (rank changed, score unchanged).
			expect(bySubject("a.rs").deltaType).toBe("changed");
			expect(bySubject("a.rs").scoreIncreased).toBe(false);
			expect(bySubject("a.rs").rankChanged).toBe(true);

			store.stopPollingRaw();
		});

		it("classifies score decrease as changed without increase flag", async () => {
			const mock = vi.spyOn(client, "getHotspots");
			mock.mockResolvedValueOnce(
				makeReport([makeEntry({ subject: "a.rs", rank: 1, score: 100 })]),
			);
			mock.mockResolvedValueOnce(
				makeReport([makeEntry({ subject: "a.rs", rank: 1, score: 80 })]),
			);
			const store = useHotspotStore();

			store.startPollingRaw(5_000);
			await vi.advanceTimersByTimeAsync(0);
			await vi.advanceTimersByTimeAsync(5_000);
			await vi.advanceTimersByTimeAsync(0);

			const entry = store.entriesWithDelta[0];
			expect(entry.deltaType).toBe("changed");
			expect(entry.scoreIncreased).toBe(false);
			expect(entry.rankChanged).toBe(false);

			store.stopPollingRaw();
		});

		it("identifies exited entries", async () => {
			const mock = vi.spyOn(client, "getHotspots");
			mock.mockResolvedValueOnce(
				makeReport([
					makeEntry({ subject: "a.rs", rank: 1, score: 100 }),
					makeEntry({ subject: "b.rs", rank: 2, score: 80 }),
				]),
			);
			mock.mockResolvedValueOnce(
				makeReport([makeEntry({ subject: "a.rs", rank: 1, score: 100 })]),
			);
			const store = useHotspotStore();

			store.startPollingRaw(5_000);
			await vi.advanceTimersByTimeAsync(0);
			await vi.advanceTimersByTimeAsync(5_000);
			await vi.advanceTimersByTimeAsync(0);

			// B exited.
			expect(store.exitedKeys.has("file:b.rs")).toBe(true);

			store.stopPollingRaw();
		});
	});

	// ── 6.6 Tab-visibility pause/resume ────────────────────────────────────
	describe("tab-visibility pause/resume", () => {
		it("pauses polling when document becomes hidden and resumes on visible via visibilitychange event", async () => {
			const mock = vi
				.spyOn(client, "getHotspots")
				.mockResolvedValue(makeReport([makeEntry()]));

			// Track the visibilitychange handler so we can fire it manually.
			let visibilityHandler: (() => void) | null = null;
			let hidden = false;

			const docMock = {
				get hidden() {
					return hidden;
				},
				addEventListener: vi.fn((_event: string, handler: () => void) => {
					visibilityHandler = handler;
				}),
				removeEventListener: vi.fn(() => {
					visibilityHandler = null;
				}),
			};
			vi.stubGlobal("document", docMock);

			const store = useHotspotStore();

			// Start polling via the public API (maps to startPollingWithVisibility).
			store.startPolling(1_000);
			expect(docMock.addEventListener).toHaveBeenCalledWith(
				"visibilitychange",
				expect.any(Function),
			);

			// Immediate first fetch.
			await vi.advanceTimersByTimeAsync(0);
			expect(store.pollState).toBe("polling");
			expect(mock).toHaveBeenCalledTimes(1);

			// Simulate tab hidden → fires visibilitychange handler.
			hidden = true;
			expect(visibilityHandler).not.toBeNull();
			visibilityHandler!();
			// Advance past 500ms debounce.
			await vi.advanceTimersByTimeAsync(500);

			// Poll state is preserved (not reset to idle).
			expect(store.pollState).toBe("polling");

			// After pause, interval ticks should NOT fire.
			await vi.advanceTimersByTimeAsync(2_000);
			expect(mock).toHaveBeenCalledTimes(1); // No additional calls.

			// Simulate tab visible → fires visibilitychange handler.
			hidden = false;
			visibilityHandler!();
			// Advance past 500ms debounce.
			await vi.advanceTimersByTimeAsync(500);

			// Resume: immediate fresh fetch.
			expect(mock).toHaveBeenCalledTimes(2);
			await vi.advanceTimersByTimeAsync(0);
			expect(store.pollState).toBe("polling");

			// Stop polling via public API (maps to stopPollingFull) — removes listener.
			store.stopPolling();
			expect(docMock.removeEventListener).toHaveBeenCalledWith(
				"visibilitychange",
				expect.any(Function),
			);
			expect(store.pollState).toBe("idle");

			// After stop, interval ticks should NOT fire.
			await vi.advanceTimersByTimeAsync(2_000);
			expect(mock).toHaveBeenCalledTimes(2);

			vi.unstubAllGlobals();
		});
	});

	// ── 6.7 First-load vs subsequent poll ──────────────────────────────────
	describe("first-load vs subsequent poll", () => {
		it("first load does not flag entries as animation candidates", async () => {
			vi.spyOn(client, "getHotspots").mockResolvedValue(
				makeReport([makeEntry({ subject: "a.rs" })]),
			);
			const store = useHotspotStore();

			await store.load();

			// After load(), isFirstLoad is set to false but entries are computed
			// based on the delta sets which are empty — all entries appear unchanged.
			const delta = store.entriesWithDelta;
			expect(delta).toHaveLength(1);
			expect(delta[0].deltaType).toBe("unchanged");
			expect(delta[0].scoreIncreased).toBe(false);
			expect(delta[0].rankChanged).toBe(false);
		});

		it("first poll (via startPolling) also marks entries as unchanged", async () => {
			vi.spyOn(client, "getHotspots").mockResolvedValue(
				makeReport([makeEntry({ subject: "a.rs" })]),
			);
			const store = useHotspotStore();

			store.startPollingRaw(60_000);
			await vi.advanceTimersByTimeAsync(0);

			// After first poll, entries are all unchanged.
			const delta = store.entriesWithDelta;
			expect(delta[0].deltaType).toBe("unchanged");

			store.stopPollingRaw();
		});
	});

	// ── Manual load ────────────────────────────────────────────────────────
	describe("manual load", () => {
		it("load() populates entries and sets pollState to idle", async () => {
			vi.spyOn(client, "getHotspots").mockResolvedValue(
				makeReport([makeEntry({ subject: "src/lib.rs" })]),
			);
			const store = useHotspotStore();

			// load() is a non-poll fetch, so pollState stays "idle".
			expect(store.pollState).toBe("idle");
			await store.load();
			expect(store.entries).toHaveLength(1);
			expect(store.entries[0].subject).toBe("src/lib.rs");
			expect(store.pollState).toBe("idle");
		});
	});
});
