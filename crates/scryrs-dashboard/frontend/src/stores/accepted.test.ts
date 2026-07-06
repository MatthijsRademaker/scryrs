import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAcceptedStore } from "@/stores/accepted";
import * as client from "@/shared/api/client";
import { useMetaStore } from "@/stores/meta";
import type {
	AcceptedItemDetail,
	AcceptedItemListRow,
} from "@/shared/api/client";

function makeListRow(
	overrides: Partial<AcceptedItemListRow> = {},
): AcceptedItemListRow {
	return {
		proposalId: "abc123",
		title: "Test Proposal",
		targetType: "docs_note",
		reviewer: "alice",
		decidedAt: "2026-07-01T00:00:00Z",
		evidenceSummary: [
			{ sourceKind: "hotspot_subject", subject: "src/lib.rs", rowIds: [1] },
		],
		publishStatus: [
			{
				surface: "rspress",
				status: "published",
				reason: "",
				path: ".devagent/docs/docs/accepted-knowledge/docs_note/abc123.md",
			},
			{
				surface: "markdown",
				status: "unknown",
				reason:
					"output root not persisted — publish path is operator-chosen at publish time",
			},
		],
		...overrides,
	};
}

function makeDetail(): AcceptedItemDetail {
	return {
		proposalId: "abc123",
		title: "Test Proposal",
		targetType: "docs_note",
		acceptedContent: "reviewed content",
		reviewer: "alice",
		rationale: "looks good",
		decidedAt: "2026-07-01T00:00:00Z",
		evidence: [],
		publishStatus: [],
	};
}

async function populateMeta(mode: "local" | "live") {
	vi.spyOn(client, "fetchMeta").mockResolvedValue({
		mode,
		repositoryPath: "/repo",
		repositoryId: null,
	});
	const meta = useMetaStore();
	await meta.ensureLoaded();
}

describe("useAcceptedStore", () => {
	beforeEach(() => {
		setActivePinia(createPinia());
		vi.restoreAllMocks();
	});

	it("fetchList dispatches API call and populates items", async () => {
		await populateMeta("local");
		const row = makeListRow();
		const spy = vi.spyOn(client, "getAcceptedList").mockResolvedValue([row]);

		const store = useAcceptedStore();
		expect(store.loading).toBe(false);
		expect(store.items).toEqual([]);
		expect(store.error).toBeNull();

		const promise = store.fetchList();
		expect(store.loading).toBe(true);

		await promise;

		expect(store.loading).toBe(false);
		expect(store.items).toEqual([row]);
		expect(store.error).toBeNull();
		expect(spy).toHaveBeenCalledTimes(1);
	});

	it("fetchList handles API error", async () => {
		await populateMeta("local");
		const apiError = new client.ApiError(
			502,
			"invalid JSON in accepted artifact",
		);
		vi.spyOn(client, "getAcceptedList").mockRejectedValue(apiError);

		const store = useAcceptedStore();
		await store.fetchList();

		expect(store.loading).toBe(false);
		expect(store.error).toBe("invalid JSON in accepted artifact");
		expect(store.items).toEqual([]);
	});

	it("fetchList handles generic error", async () => {
		await populateMeta("local");
		vi.spyOn(client, "getAcceptedList").mockRejectedValue(
			new Error("network failure"),
		);

		const store = useAcceptedStore();
		await store.fetchList();

		expect(store.loading).toBe(false);
		expect(store.error).toBe("network failure");
		expect(store.items).toEqual([]);
	});

	it("fetchList skips API call in live mode", async () => {
		await populateMeta("live");
		const spy = vi.spyOn(client, "getAcceptedList");

		const store = useAcceptedStore();
		await store.fetchList();

		expect(spy).not.toHaveBeenCalled();
		expect(store.loading).toBe(false);
		expect(store.items).toEqual([]);
	});

	it("fetchDetail dispatches API call and populates selectedItem", async () => {
		await populateMeta("local");
		const detail = makeDetail();
		const spy = vi.spyOn(client, "getAcceptedDetail").mockResolvedValue(detail);

		const store = useAcceptedStore();
		expect(store.detailLoading).toBe(false);
		expect(store.selectedItem).toBeNull();
		expect(store.detailError).toBeNull();

		const promise = store.fetchDetail("abc123");
		expect(store.detailLoading).toBe(true);

		await promise;

		expect(store.detailLoading).toBe(false);
		expect(store.selectedItem).toEqual(detail);
		expect(store.detailError).toBeNull();
		expect(spy).toHaveBeenCalledWith("abc123");
	});

	it("fetchDetail handles API error", async () => {
		await populateMeta("local");
		const apiError = new client.ApiError(
			404,
			"proposal not found among accepted decisions",
		);
		vi.spyOn(client, "getAcceptedDetail").mockRejectedValue(apiError);

		const store = useAcceptedStore();
		await store.fetchDetail("nonexistent");

		expect(store.detailLoading).toBe(false);
		expect(store.detailError).toBe(
			"proposal not found among accepted decisions",
		);
		expect(store.selectedItem).toBeNull();
	});

	it("fetchDetail skips API call in live mode", async () => {
		await populateMeta("live");
		const spy = vi.spyOn(client, "getAcceptedDetail");

		const store = useAcceptedStore();
		await store.fetchDetail("abc123");

		expect(spy).not.toHaveBeenCalled();
		expect(store.detailLoading).toBe(false);
		expect(store.selectedItem).toBeNull();
	});
});

describe("navigationForMode accepted entry", () => {
	// We import navigationForMode lazily to avoid circular dependency issues.
	it("includes Accepted in local mode", async () => {
		const { navigationForMode } = await import("@/shared/lib/dashboard-mode");
		const nav = navigationForMode("local");
		const acceptedItem = nav.find((item) => item.to === "/accepted");
		expect(acceptedItem).toBeDefined();
		expect(acceptedItem?.label).toBe("Accepted");
		expect(acceptedItem?.match).toEqual(["accepted", "accepted-detail"]);
	});

	it("excludes Accepted in live mode", async () => {
		const { navigationForMode } = await import("@/shared/lib/dashboard-mode");
		const nav = navigationForMode("live");
		const acceptedItem = nav.find((item) => item.to === "/accepted");
		expect(acceptedItem).toBeUndefined();
	});
});
