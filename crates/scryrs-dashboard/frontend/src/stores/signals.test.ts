import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useSignalStore } from "@/stores/signals";

interface MockMessage {
	data: string;
	lastEventId: string;
}

class MockEventSource {
	static instances: MockEventSource[] = [];

	public readonly url: string;
	public closed = false;
	public onopen: (() => void) | null = null;
	public onerror: (() => void) | null = null;
	public onmessage: ((event: MockMessage) => void) | null = null;

	constructor(url: string) {
		this.url = url;
		MockEventSource.instances.push(this);
	}

	close() {
		this.closed = true;
	}

	emitOpen() {
		this.onopen?.();
	}

	emitError() {
		this.onerror?.();
	}

	emitMessage(id: number, data: Record<string, unknown>) {
		this.onmessage?.({ data: JSON.stringify(data), lastEventId: String(id) });
	}
}

describe("useSignalStore", () => {
	beforeEach(() => {
		vi.useFakeTimers();
		MockEventSource.instances = [];
		setActivePinia(createPinia());
		vi.stubGlobal("EventSource", MockEventSource);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		vi.useRealTimers();
	});

	it("reconnects with the last seen id and ignores replay duplicates", async () => {
		const store = useSignalStore();

		store.start();
		expect(store.connectionState).toBe("connecting");
		expect(MockEventSource.instances[0]?.url).toBe("/api/signals?after=0");

		MockEventSource.instances[0]?.emitOpen();
		expect(store.connectionState).toBe("connected");

		MockEventSource.instances[0]?.emitMessage(41, {
			repositoryId: "repo-a",
			subjectKind: "file",
			subject: "src/main.rs",
			score: 10,
			delta: 1,
			window: "cumulative",
			threshold: 10,
			evidenceRowIds: [41],
			createdAt: "2026-06-29T19:00:00Z",
		});

		expect(store.signals).toHaveLength(1);
		expect(store.lastSeenId).toBe(41);

		MockEventSource.instances[0]?.emitError();
		expect(store.connectionState).toBe("reconnecting");
		expect(MockEventSource.instances[0]?.closed).toBe(true);

		await vi.advanceTimersByTimeAsync(1_000);
		expect(MockEventSource.instances[1]?.url).toBe("/api/signals?after=41");

		MockEventSource.instances[1]?.emitOpen();
		MockEventSource.instances[1]?.emitMessage(41, {
			repositoryId: "repo-a",
			subjectKind: "file",
			subject: "src/main.rs",
			score: 10,
			delta: 1,
			window: "cumulative",
			threshold: 10,
			evidenceRowIds: [41],
			createdAt: "2026-06-29T19:00:00Z",
		});
		MockEventSource.instances[1]?.emitMessage(42, {
			repositoryId: "repo-a",
			subjectKind: "file",
			subject: "src/lib.rs",
			score: 11,
			delta: 1,
			window: "cumulative",
			threshold: 10,
			evidenceRowIds: [42],
			createdAt: "2026-06-29T19:00:01Z",
		});

		expect(store.signals.map((signal) => signal.id)).toEqual([41, 42]);
		expect(store.connectionState).toBe("connected");
	});
});
