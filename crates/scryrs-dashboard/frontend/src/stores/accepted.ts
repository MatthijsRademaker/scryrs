import { defineStore } from "pinia";
import { ref } from "vue";
import {
	getAcceptedDetail,
	getAcceptedList,
	type AcceptedItemDetail,
	type AcceptedItemListRow,
} from "@/shared/api/client";
import { useMetaStore } from "@/stores/meta";

export const useAcceptedStore = defineStore("accepted", () => {
	const items = ref<AcceptedItemListRow[]>([]);
	const loading = ref(false);
	const error = ref<string | null>(null);

	const selectedItem = ref<AcceptedItemDetail | null>(null);
	const detailLoading = ref(false);
	const detailError = ref<string | null>(null);

	async function fetchList() {
		const meta = useMetaStore();
		if (meta.isLiveMode) return;

		loading.value = true;
		error.value = null;
		try {
			items.value = await getAcceptedList();
		} catch (unknownError) {
			error.value =
				unknownError instanceof Error
					? unknownError.message
					: "Accepted knowledge data could not be read";
		} finally {
			loading.value = false;
		}
	}

	async function fetchDetail(proposalId: string) {
		const meta = useMetaStore();
		if (meta.isLiveMode) return;

		detailLoading.value = true;
		detailError.value = null;
		try {
			selectedItem.value = await getAcceptedDetail(proposalId);
		} catch (unknownError) {
			detailError.value =
				unknownError instanceof Error
					? unknownError.message
					: "Accepted knowledge detail could not be read";
		} finally {
			detailLoading.value = false;
		}
	}

	return {
		items,
		loading,
		error,
		selectedItem,
		detailLoading,
		detailError,
		fetchList,
		fetchDetail,
	};
});
