import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { getEvents, type TraceEventItem } from "@/shared/api/client";

export const useEventStore = defineStore("events", () => {
  const events = ref<TraceEventItem[]>([]);
  const nextCursor = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const distribution = computed(() => events.value.reduce<Record<string, number>>((acc, event) => {
    acc[event.eventType] = (acc[event.eventType] ?? 0) + 1;
    return acc;
  }, {}));

  async function load(params: { sessionId?: string | null; cursor?: string | null } = {}) {
    loading.value = true;
    error.value = null;
    try {
      const page = await getEvents({ limit: 100, cursor: params.cursor, sessionId: params.sessionId });
      events.value = params.cursor ? [...events.value, ...page.events] : page.events;
      nextCursor.value = page.nextCursor;
    } catch (unknownError) {
      error.value = unknownError instanceof Error ? unknownError.message : "Event data could not be read";
    } finally {
      loading.value = false;
    }
  }

  return { events, nextCursor, distribution, loading, error, load };
});
