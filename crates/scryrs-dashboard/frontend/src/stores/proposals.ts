import { defineStore } from "pinia";
import { ref } from "vue";
import {
	getProposal,
	getProposals,
	type ProposalDetail,
	type ProposalListRow,
} from "@/shared/api/client";

export const useProposalStore = defineStore("proposals", () => {
	const rows = ref<ProposalListRow[]>([]);
	const detail = ref<ProposalDetail | null>(null);
	const loading = ref(false);
	const error = ref<string | null>(null);

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

	return { rows, detail, loading, error, loadProposals, loadProposal };
});
