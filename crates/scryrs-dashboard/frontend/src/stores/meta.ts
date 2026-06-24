import { defineStore } from "pinia";
import { ref } from "vue";
import { fetchMeta } from "@/shared/api/client";

export const useMetaStore = defineStore("meta", () => {
  const repositoryPath = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // Fetch the repository root once; subsequent calls are no-ops so any view can
  // request it on mount without triggering duplicate network round-trips.
  async function ensureLoaded() {
    if (repositoryPath.value !== null || loading.value) return;
    loading.value = true;
    error.value = null;
    try {
      const meta = await fetchMeta();
      repositoryPath.value = meta.repositoryPath;
    } catch (unknownError) {
      error.value = unknownError instanceof Error ? unknownError.message : "Repository metadata could not be read";
    } finally {
      loading.value = false;
    }
  }

  return { repositoryPath, loading, error, ensureLoaded };
});
