import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useRouteStore } from "@/stores/routes";
import * as client from "@/shared/api/client";
import { navigationForMode } from "@/shared/lib/dashboard-mode";

function makeHint(overrides: Partial<client.RouteHintItem> = {}) {
	return {
		routeId: "file:auth",
		target: "file:auth",
		label: "auth",
		rank: 1,
		reason:
			"Route 'auth' (file:auth): 1 evidence link(s), subject kind file, load target file; query match on label, subject",
		evidence: [],
		...overrides,
	};
}

describe("useRouteStore", () => {
	beforeEach(() => {
		setActivePinia(createPinia());
		vi.restoreAllMocks();
	});

	it("search dispatches API call and populates hints", async () => {
		const hint = makeHint();
		const mockDoc: client.RouteHintDocument = {
			schemaVersion: "1.0.0",
			hints: [hint],
		};
		const spy = vi.spyOn(client, "getRouteHints").mockResolvedValue(mockDoc);

		const store = useRouteStore();
		expect(store.loading).toBe(false);
		expect(store.hints).toEqual([]);
		expect(store.error).toBeNull();

		const promise = store.search("auth");
		expect(store.loading).toBe(true);

		await promise;

		expect(store.loading).toBe(false);
		expect(store.hints).toEqual([hint]);
		expect(store.error).toBeNull();
		expect(store.query).toBe("auth");
		expect(spy).toHaveBeenCalledWith("auth");
	});

	it("search handles API error", async () => {
		const apiError = new client.ApiError(404, "route artifact not found");
		vi.spyOn(client, "getRouteHints").mockRejectedValue(apiError);

		const store = useRouteStore();
		await store.search("auth");

		expect(store.loading).toBe(false);
		expect(store.error).toBe("route artifact not found");
		expect(store.hints).toEqual([]);
	});

	it("search handles generic error", async () => {
		vi.spyOn(client, "getRouteHints").mockRejectedValue(
			new Error("network failure"),
		);

		const store = useRouteStore();
		await store.search("auth");

		expect(store.loading).toBe(false);
		expect(store.error).toBe("network failure");
		expect(store.hints).toEqual([]);
	});

	it("search skips API call for empty query", () => {
		const spy = vi.spyOn(client, "getRouteHints");

		const store = useRouteStore();
		store.search("");

		expect(spy).not.toHaveBeenCalled();
		expect(store.hints).toEqual([]);
	});

	it("search skips API call for whitespace-only query", () => {
		const spy = vi.spyOn(client, "getRouteHints");

		const store = useRouteStore();
		store.search("   ");

		expect(spy).not.toHaveBeenCalled();
		expect(store.hints).toEqual([]);
	});

	it("search sets query even when empty (frontend gating)", () => {
		const store = useRouteStore();
		store.search("");
		expect(store.query).toBe("");
	});

	it("getRouteHints encodes query parameter", async () => {
		const spy = vi
			.spyOn(client, "getRouteHints")
			.mockResolvedValue({ schemaVersion: "1.0.0", hints: [] });

		const store = useRouteStore();
		await store.search("auth & stuff");

		expect(spy).toHaveBeenCalledWith("auth & stuff");
	});
});

describe("navigationForMode route entry", () => {
	it("includes Routes in local mode", () => {
		const nav = navigationForMode("local");
		const routeItem = nav.find((item) => item.to === "/routes");
		expect(routeItem).toBeDefined();
		expect(routeItem?.label).toBe("Routes");
	});

	it("excludes Routes in live mode", () => {
		const nav = navigationForMode("live");
		const routeItem = nav.find((item) => item.to === "/routes");
		expect(routeItem).toBeUndefined();
	});
});
