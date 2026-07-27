import type { ProposalDetail } from "@/shared/api/client";

export type ProposalDetailContent = {
	type: "markdown" | "semantic_graph_grouping" | "memory_patch" | "unknown";
	text: string;
};

export function showReviewForm(
	canReview: boolean,
	detail: ProposalDetail | null,
): boolean {
	return canReview && !!detail && !detail.reviewDecision;
}

export function reviewInputsValid(
	reviewer: string,
	rationale: string,
	decidedAt: string,
): boolean {
	return (
		reviewer.trim().length > 0 &&
		rationale.trim().length > 0 &&
		decidedAt.trim().length > 0
	);
}

export function contentDisplay(content: unknown): ProposalDetailContent {
	if (typeof content === "string") {
		return { type: "markdown", text: content };
	}
	if (content && typeof content === "object") {
		const obj = content as Record<string, unknown>;
		if (Array.isArray(obj.sourceNodeIds)) {
			return {
				type: "semantic_graph_grouping",
				text: JSON.stringify(
					{
						sourceNodeIds: obj.sourceNodeIds,
						targetGroupNodeId: obj.targetGroupNodeId,
						targetGroupLabel: obj.targetGroupLabel,
					},
					null,
					2,
				),
			};
		}
		return { type: "memory_patch", text: JSON.stringify(obj, null, 2) };
	}
	return { type: "unknown", text: JSON.stringify(content, null, 2) };
}

export function canEditReviewedContent(
	canReview: boolean,
	detail: ProposalDetail | null,
): boolean {
	return (
		showReviewForm(canReview, detail) &&
		contentDisplay(detail?.proposedContent).type === "markdown"
	);
}

export function reviewedContentPayload(
	canReview: boolean,
	detail: ProposalDetail | null,
	reviewedContent: string,
): string | undefined {
	if (!canEditReviewedContent(canReview, detail)) {
		return undefined;
	}
	if (typeof detail?.proposedContent !== "string") {
		return undefined;
	}
	return reviewedContent === detail.proposedContent
		? undefined
		: reviewedContent;
}

export function proposalReviewErrorLabel(status: number): string {
	if (status === 401 || status === 403) return "Authorization failed";
	if (status === 400 || status === 413 || status === 422) {
		return "Validation failed";
	}
	if (status === 409) return "Review conflict";
	if (status === 502) return "Live server failed";
	return "Review failed";
}
