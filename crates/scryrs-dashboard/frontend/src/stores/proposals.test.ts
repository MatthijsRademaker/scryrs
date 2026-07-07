import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useProposalStore } from "@/stores/proposals";
import * as client from "@/shared/api/client";

function makeDetail(
	overrides: Partial<client.ProposalDetail> = {},
): client.ProposalDetail {
	return {
		schemaVersion: "1.0.0",
		id: "abc123",
		targetType: "docs_note",
		title: "Proposal",
		rationale: "Explain it",
		proposedContent: "# Draft",
		evidence: [],
		createdAt: "2026-07-01T00:00:00Z",
		reviewDecision: null,
		...overrides,
	};
}

describe("useProposalStore", () => {
	beforeEach(() => {
		setActivePinia(createPinia());
		vi.restoreAllMocks();
	});

	it("acceptProposal posts review data and refreshes detail and rows", async () => {
		const rows: client.ProposalListRow[] = [
			{
				proposalId: "abc123",
				title: "Proposal",
				targetType: "docs_note",
				createdAt: "2026-07-01T00:00:00Z",
				state: "accepted",
			},
		];
		const updatedDetail = makeDetail({
			reviewDecision: {
				reviewer: "alice",
				outcome: "accepted",
				decidedAt: "2026-07-03T00:00:00Z",
				rationale: "looks good",
				acceptedContent: "# Final",
				targetType: "docs_note",
			},
		});
		const acceptSpy = vi
			.spyOn(client, "acceptProposalReview")
			.mockResolvedValue({ outcome: "accepted" });
		const rowsSpy = vi.spyOn(client, "getProposals").mockResolvedValue(rows);
		const detailSpy = vi
			.spyOn(client, "getProposal")
			.mockResolvedValue(updatedDetail);

		const store = useProposalStore();
		const promise = store.acceptProposal({
			proposalId: "abc123",
			reviewer: "alice",
			rationale: "looks good",
			decidedAt: "2026-07-03T00:00:00Z",
			reviewedContent: "# Final",
		});
		expect(store.reviewLoading).toBe(true);

		await promise;

		expect(acceptSpy).toHaveBeenCalledWith("abc123", {
			reviewer: "alice",
			rationale: "looks good",
			decidedAt: "2026-07-03T00:00:00Z",
			reviewedContent: "# Final",
		});
		expect(rowsSpy).toHaveBeenCalledTimes(1);
		expect(detailSpy).toHaveBeenCalledWith("abc123");
		expect(store.reviewLoading).toBe(false);
		expect(store.reviewError).toBeNull();
		expect(store.reviewErrorStatus).toBe(0);
		expect(store.reviewSuccess).toBe("Proposal accepted.");
		expect(store.rows).toEqual(rows);
		expect(store.detail).toEqual(updatedDetail);
	});

	it("rejectProposal refreshes detail and rows after a successful reject", async () => {
		const rows: client.ProposalListRow[] = [
			{
				proposalId: "abc123",
				title: "Proposal",
				targetType: "docs_note",
				createdAt: "2026-07-01T00:00:00Z",
				state: "rejected",
			},
		];
		const updatedDetail = makeDetail({
			reviewDecision: {
				reviewer: "alice",
				outcome: "rejected",
				decidedAt: "2026-07-03T00:00:00Z",
				rationale: "off-scope",
			},
		});
		const rejectSpy = vi
			.spyOn(client, "rejectProposalReview")
			.mockResolvedValue({ outcome: "rejected" });
		const rowsSpy = vi.spyOn(client, "getProposals").mockResolvedValue(rows);
		const detailSpy = vi
			.spyOn(client, "getProposal")
			.mockResolvedValue(updatedDetail);

		const store = useProposalStore();
		await store.rejectProposal({
			proposalId: "abc123",
			reviewer: "alice",
			rationale: "off-scope",
			decidedAt: "2026-07-03T00:00:00Z",
		});

		expect(rejectSpy).toHaveBeenCalledWith("abc123", {
			reviewer: "alice",
			rationale: "off-scope",
			decidedAt: "2026-07-03T00:00:00Z",
		});
		expect(rowsSpy).toHaveBeenCalledTimes(1);
		expect(detailSpy).toHaveBeenCalledWith("abc123");
		expect(store.reviewLoading).toBe(false);
		expect(store.reviewError).toBeNull();
		expect(store.reviewErrorStatus).toBe(0);
		expect(store.reviewSuccess).toBe("Proposal rejected.");
		expect(store.rows).toEqual(rows);
		expect(store.detail).toEqual(updatedDetail);
	});

	it("acceptProposal surfaces bad-gateway failures without refreshing state", async () => {
		const acceptSpy = vi
			.spyOn(client, "acceptProposalReview")
			.mockRejectedValue(new client.ApiError(502, "artifact write failed"));
		const rowsSpy = vi.spyOn(client, "getProposals");
		const detailSpy = vi.spyOn(client, "getProposal");

		const store = useProposalStore();
		await store.acceptProposal({
			proposalId: "abc123",
			reviewer: "alice",
			rationale: "looks good",
			decidedAt: "2026-07-03T00:00:00Z",
			reviewedContent: "# Final",
		});

		expect(acceptSpy).toHaveBeenCalledWith("abc123", {
			reviewer: "alice",
			rationale: "looks good",
			decidedAt: "2026-07-03T00:00:00Z",
			reviewedContent: "# Final",
		});
		expect(rowsSpy).not.toHaveBeenCalled();
		expect(detailSpy).not.toHaveBeenCalled();
		expect(store.reviewLoading).toBe(false);
		expect(store.reviewError).toBe("artifact write failed");
		expect(store.reviewErrorStatus).toBe(502);
		expect(store.reviewSuccess).toBeNull();
	});

	it("rejectProposal surfaces ApiError status without refreshing state", async () => {
		const rejectSpy = vi
			.spyOn(client, "rejectProposalReview")
			.mockRejectedValue(
				new client.ApiError(409, "conflicting terminal decision"),
			);
		const rowsSpy = vi.spyOn(client, "getProposals");
		const detailSpy = vi.spyOn(client, "getProposal");

		const store = useProposalStore();
		await store.rejectProposal({
			proposalId: "abc123",
			reviewer: "alice",
			rationale: "off-scope",
			decidedAt: "2026-07-03T00:00:00Z",
		});

		expect(rejectSpy).toHaveBeenCalledWith("abc123", {
			reviewer: "alice",
			rationale: "off-scope",
			decidedAt: "2026-07-03T00:00:00Z",
		});
		expect(rowsSpy).not.toHaveBeenCalled();
		expect(detailSpy).not.toHaveBeenCalled();
		expect(store.reviewLoading).toBe(false);
		expect(store.reviewError).toBe("conflicting terminal decision");
		expect(store.reviewErrorStatus).toBe(409);
		expect(store.reviewSuccess).toBeNull();
	});
});
