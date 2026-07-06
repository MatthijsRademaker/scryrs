import { defineStore } from "pinia";
import { ref } from "vue";
import {
	ApiError,
	getRouteHints,
	type RouteHintItem,
} from "@/shared/api/client";

export const useRouteStore = defineStore("routes", () => {
	const hints = ref<RouteHintItem[]>([]);
	const loading = ref(false);
	const error = ref<string | null>(null);
	const errorStatus = ref<number>(0);
	const query = ref("");

	async function search(q: string) {
		query.value = q;

		// Defense-in-depth: skip API call when query is empty.
		if (q.trim().length === 0) {
			return;
		}

		loading.value = true;
		error.value = null;
		errorStatus.value = 0;
		try {
			const doc = await getRouteHints(q);
			hints.value = doc.hints;
		} catch (unknownError) {
			const message =
				unknownError instanceof Error
					? unknownError.message
					: "Route explain request failed";
			error.value = message;
			errorStatus.value =
				unknownError instanceof ApiError ? unknownError.status : 0;
			hints.value = [];
		} finally {
			loading.value = false;
		}
	}

	return { hints, loading, error, errorStatus, query, search };
});
