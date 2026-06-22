import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { getHotspots, type HotspotsReport } from "@/shared/api/client";

export const useHotspotStore = defineStore("hotspots", () => {
  const report = ref<HotspotsReport | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const entries = computed(() => report.value?.entries ?? []);

  async function load() {
    loading.value = true;
    error.value = null;
    try {
      report.value = await getHotspots();
    } catch (unknownError) {
      error.value = unknownError instanceof Error ? unknownError.message : "Hotspot data could not be read";
    } finally {
      loading.value = false;
    }
  }

  return { report, entries, loading, error, load };
});
