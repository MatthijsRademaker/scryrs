import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import * as client from "@/shared/api/client";
import { useEventStore } from "@/stores/events";
import { useSessionStore } from "@/stores/sessions";

const summary: client.SessionSummary = {
	sessionId: "live-s1",
	startedAt: "2026-07-05T10:00:00Z",
	endedAt: null,
	eventCount: 2,
	source: "pi",
};

function event(eventId: number, sessionId = "live-s1"): client.TraceEventItem {
	return {
		eventId,
		sessionId,
		eventType: "FileOpened",
		timestamp: "2026-07-05T10:00:00Z",
		subjectKind: "file",
		subject: "src/main.rs",
		payload: { path: "src/main.rs" },
	};
}

describe("live-compatible session and event stores", () => {
	beforeEach(() => {
		setActivePinia(createPinia());
		vi.restoreAllMocks();
	});

	it("loads session summaries and detail through same-origin clients", async () => {
		vi.spyOn(client, "getSessions").mockResolvedValue([summary]);
		vi.spyOn(client, "getSession").mockResolvedValue({
			session: summary,
			events: [event(1), event(2)],
		});
		const store = useSessionStore();

		await store.loadSessions();
		await store.loadSession("live-s1");

		expect(store.sessions).toEqual([summary]);
		expect(store.detail?.events.map((item) => item.eventId)).toEqual([1, 2]);
		expect(client.getSessions).toHaveBeenCalledOnce();
		expect(client.getSession).toHaveBeenCalledWith("live-s1");
	});

	it("clears stale detail while direct-route session request is loading", async () => {
		let resolveRequest: ((detail: client.SessionDetail) => void) | undefined;
		vi.spyOn(client, "getSession")
			.mockResolvedValueOnce({ session: summary, events: [event(1)] })
			.mockImplementationOnce(
				() =>
					new Promise((resolve) => {
						resolveRequest = resolve;
					}),
			);
		const store = useSessionStore();
		await store.loadSession("live-s1");

		const pending = store.loadSession("live-s2");
		expect(store.loading).toBe(true);
		expect(store.detail).toBeNull();
		resolveRequest?.({
			session: { ...summary, sessionId: "live-s2" },
			events: [event(2, "live-s2")],
		});
		await pending;
		expect(store.detail?.session.sessionId).toBe("live-s2");
	});

	it("appends cursor pages and resets events when filter changes", async () => {
		const api = vi
			.spyOn(client, "getEvents")
			.mockResolvedValueOnce({ events: [event(3), event(2)], nextCursor: "2" })
			.mockResolvedValueOnce({ events: [event(1)], nextCursor: null })
			.mockResolvedValueOnce({
				events: [event(4, "live-s2")],
				nextCursor: null,
			});
		const store = useEventStore();

		await store.load();
		await store.load({ cursor: store.nextCursor });
		expect(store.events.map((item) => item.eventId)).toEqual([3, 2, 1]);

		await store.load({ sessionId: "live-s2" });
		expect(store.events.map((item) => item.eventId)).toEqual([4]);
		expect(api).toHaveBeenNthCalledWith(2, {
			limit: 100,
			cursor: "2",
			sessionId: undefined,
		});
		expect(api).toHaveBeenNthCalledWith(3, {
			limit: 100,
			cursor: undefined,
			sessionId: "live-s2",
		});
	});

	it("surfaces upstream errors without fake success data", async () => {
		vi.spyOn(client, "getSessions").mockRejectedValue(
			new client.ApiError(502, "live sessions upstream request failed"),
		);
		vi.spyOn(client, "getEvents").mockRejectedValue(
			new client.ApiError(502, "live events upstream request failed"),
		);
		const sessions = useSessionStore();
		const events = useEventStore();

		await sessions.loadSessions();
		await events.load();

		expect(sessions.sessions).toEqual([]);
		expect(sessions.error).toContain("upstream request failed");
		expect(events.events).toEqual([]);
		expect(events.error).toContain("upstream request failed");
	});
});
