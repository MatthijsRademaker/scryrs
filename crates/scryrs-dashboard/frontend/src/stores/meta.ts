import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { fetchMeta, type DashboardMode } from "@/shared/api/client";

export const useMetaStore = defineStore("meta", () => {
	const mode = ref<DashboardMode | null>(null);
	const repositoryPath = ref<string | null>(null);
	const repositoryId = ref<string | null>(null);
	const proposalReadsAvailable = ref(false);
	const proposalReviewWritesAvailable = ref(false);
	const loading = ref(false);
	const error = ref<string | null>(null);
	const isLiveMode = computed(() => mode.value === "live");

	async function ensureLoaded() {
		if (repositoryPath.value !== null || loading.value) return;
		loading.value = true;
		error.value = null;
		try {
			const meta = await fetchMeta();
			mode.value = meta.mode;
			repositoryPath.value = meta.repositoryPath;
			repositoryId.value = meta.repositoryId ?? null;
			proposalReadsAvailable.value = meta.proposalReadsAvailable;
			proposalReviewWritesAvailable.value = meta.proposalReviewWritesAvailable;
		} catch (unknownError) {
			error.value =
				unknownError instanceof Error
					? unknownError.message
					: "Repository metadata could not be read";
		} finally {
			loading.value = false;
		}
	}

	return {
		mode,
		repositoryPath,
		repositoryId,
		proposalReadsAvailable,
		proposalReviewWritesAvailable,
		isLiveMode,
		loading,
		error,
		ensureLoaded,
	};
});
