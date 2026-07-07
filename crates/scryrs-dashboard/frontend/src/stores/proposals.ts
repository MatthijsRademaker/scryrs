import { defineStore } from "pinia";
import { ref } from "vue";
import {
	acceptProposalReview,
	ApiError,
	getProposal,
	getProposals,
	rejectProposalReview,
	type ProposalDetail,
	type ProposalListRow,
	type ProposalReviewSubmission,
} from "@/shared/api/client";

export const useProposalStore = defineStore("proposals", () => {
	const rows = ref<ProposalListRow[]>([]);
	const detail = ref<ProposalDetail | null>(null);
	const loading = ref(false);
	const error = ref<string | null>(null);
	const reviewLoading = ref(false);
	const reviewError = ref<string | null>(null);
	const reviewErrorStatus = ref(0);
	const reviewSuccess = ref<string | null>(null);

	async function loadProposals() {
		loading.value = true;
		error.value = null;
		try {
			rows.value = await getProposals();
		} catch (unknownError) {
			error.value =
				unknownError instanceof Error
					? unknownError.message
					: "Proposal data could not be read";
		} finally {
			loading.value = false;
		}
	}

	async function loadProposal(proposalId: string) {
		loading.value = true;
		error.value = null;
		try {
			detail.value = await getProposal(proposalId);
		} catch (unknownError) {
			error.value =
				unknownError instanceof Error
					? unknownError.message
					: "Proposal detail could not be read";
		} finally {
			loading.value = false;
		}
	}

	async function refreshAfterReview(proposalId: string) {
		const [nextRows, nextDetail] = await Promise.all([
			getProposals(),
			getProposal(proposalId),
		]);
		rows.value = nextRows;
		detail.value = nextDetail;
	}

	async function acceptProposal(
		payload: ProposalReviewSubmission & { proposalId: string },
	) {
		reviewLoading.value = true;
		reviewError.value = null;
		reviewErrorStatus.value = 0;
		reviewSuccess.value = null;
		try {
			await acceptProposalReview(payload.proposalId, {
				reviewer: payload.reviewer,
				rationale: payload.rationale,
				decidedAt: payload.decidedAt,
				reviewedContent: payload.reviewedContent,
			});
			await refreshAfterReview(payload.proposalId);
			reviewSuccess.value = "Proposal accepted.";
		} catch (unknownError) {
			reviewError.value =
				unknownError instanceof Error
					? unknownError.message
					: "Proposal review failed";
			reviewErrorStatus.value =
				unknownError instanceof ApiError ? unknownError.status : 0;
		} finally {
			reviewLoading.value = false;
		}
	}

	async function rejectProposal(
		payload: Omit<ProposalReviewSubmission, "reviewedContent"> & {
			proposalId: string;
		},
	) {
		reviewLoading.value = true;
		reviewError.value = null;
		reviewErrorStatus.value = 0;
		reviewSuccess.value = null;
		try {
			await rejectProposalReview(payload.proposalId, {
				reviewer: payload.reviewer,
				rationale: payload.rationale,
				decidedAt: payload.decidedAt,
			});
			await refreshAfterReview(payload.proposalId);
			reviewSuccess.value = "Proposal rejected.";
		} catch (unknownError) {
			reviewError.value =
				unknownError instanceof Error
					? unknownError.message
					: "Proposal review failed";
			reviewErrorStatus.value =
				unknownError instanceof ApiError ? unknownError.status : 0;
		} finally {
			reviewLoading.value = false;
		}
	}

	return {
		rows,
		detail,
		loading,
		error,
		reviewLoading,
		reviewError,
		reviewErrorStatus,
		reviewSuccess,
		loadProposals,
		loadProposal,
		acceptProposal,
		rejectProposal,
	};
});
