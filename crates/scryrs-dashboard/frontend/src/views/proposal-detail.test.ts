import { describe, expect, it } from "vitest";
import type { ProposalDetail } from "@/shared/api/client";
import {
	canEditReviewedContent,
	contentDisplay,
	proposalReviewErrorLabel,
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
	it("shows the review form only for pending proposals with write capability", () => {
		expect(showReviewForm(true, makeDetail())).toBe(true);
		expect(showReviewForm(false, makeDetail())).toBe(false);
		expect(
			showReviewForm(
				true,
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
		expect(canEditReviewedContent(true, makeDetail())).toBe(true);
		expect(
			canEditReviewedContent(
				true,
				makeDetail({
					targetType: "memory_patch",
					proposedContent: { patch: "alpha" },
				}),
			),
		).toBe(false);
		expect(canEditReviewedContent(false, makeDetail())).toBe(false);
	});

	it("omits unchanged reviewed content and rejects structured target overrides", () => {
		const markdownDetail = makeDetail({ proposedContent: "# Draft" });
		expect(
			reviewedContentPayload(true, markdownDetail, "# Draft"),
		).toBeUndefined();
		expect(reviewedContentPayload(true, markdownDetail, "# Final")).toBe(
			"# Final",
		);
		expect(
			reviewedContentPayload(
				true,
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

	it("labels authorization, validation, conflict, and upstream errors", () => {
		expect(proposalReviewErrorLabel(403)).toBe("Authorization failed");
		expect(proposalReviewErrorLabel(422)).toBe("Validation failed");
		expect(proposalReviewErrorLabel(409)).toBe("Review conflict");
		expect(proposalReviewErrorLabel(502)).toBe("Live server failed");
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
