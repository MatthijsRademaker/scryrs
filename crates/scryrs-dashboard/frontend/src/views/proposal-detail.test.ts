import { describe, expect, it } from "vitest";
import type { ProposalDetail } from "@/shared/api/client";
import {
	canEditReviewedContent,
	contentDisplay,
	reviewedContentPayload,
	reviewInputsValid,
	showReviewForm,
} from "@/views/proposal-detail";

function makeDetail(overrides: Partial<ProposalDetail> = {}): ProposalDetail {
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

describe("proposal detail review helpers", () => {
	it("shows the review form only for pending local proposals", () => {
		expect(showReviewForm(false, makeDetail())).toBe(true);
		expect(showReviewForm(true, makeDetail())).toBe(false);
		expect(
			showReviewForm(
				false,
				makeDetail({
					reviewDecision: {
						reviewer: "alice",
						outcome: "accepted",
						decidedAt: "2026-07-03T00:00:00Z",
						rationale: "looks good",
					},
				}),
			),
		).toBe(false);
	});

	it("requires non-empty reviewer metadata before review actions enable", () => {
		expect(
			reviewInputsValid("alice", "looks good", "2026-07-03T00:00:00Z"),
		).toBe(true);
		expect(reviewInputsValid(" ", "looks good", "2026-07-03T00:00:00Z")).toBe(
			false,
		);
		expect(reviewInputsValid("alice", "", "2026-07-03T00:00:00Z")).toBe(false);
		expect(reviewInputsValid("alice", "looks good", "  ")).toBe(false);
	});

	it("allows reviewed content edits only for markdown-backed pending proposals", () => {
		expect(canEditReviewedContent(false, makeDetail())).toBe(true);
		expect(
			canEditReviewedContent(
				false,
				makeDetail({
					targetType: "memory_patch",
					proposedContent: { patch: "alpha" },
				}),
			),
		).toBe(false);
		expect(canEditReviewedContent(true, makeDetail())).toBe(false);
	});

	it("omits unchanged reviewed content and rejects structured target overrides", () => {
		const markdownDetail = makeDetail({ proposedContent: "# Draft" });
		expect(
			reviewedContentPayload(false, markdownDetail, "# Draft"),
		).toBeUndefined();
		expect(reviewedContentPayload(false, markdownDetail, "# Final")).toBe(
			"# Final",
		);
		expect(
			reviewedContentPayload(
				false,
				makeDetail({
					targetType: "semantic_graph_grouping",
					proposedContent: {
						sourceNodeIds: ["n1"],
						targetGroupNodeId: "group:auth",
						targetGroupLabel: "auth",
					},
				}),
				"override",
			),
		).toBeUndefined();
	});

	it("formats structured proposal content for display", () => {
		expect(contentDisplay("# Draft")).toEqual({
			type: "markdown",
			text: "# Draft",
		});
		expect(
			contentDisplay({
				sourceNodeIds: ["n1"],
				targetGroupNodeId: "group:auth",
				targetGroupLabel: "auth",
			}),
		).toEqual({
			type: "semantic_graph_grouping",
			text: JSON.stringify(
				{
					sourceNodeIds: ["n1"],
					targetGroupNodeId: "group:auth",
					targetGroupLabel: "auth",
				},
				null,
				2,
			),
		});
	});
});
