import { defineStore } from "pinia";
import { ref } from "vue";
import { getSession, getSessions, type SessionDetail, type SessionSummary } from "@/shared/api/client";

export const useSessionStore = defineStore("sessions", () => {
  const sessions = ref<SessionSummary[]>([]);
  const detail = ref<SessionDetail | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function loadSessions() {
    loading.value = true;
    error.value = null;
    try {
      sessions.value = await getSessions();
    } catch (unknownError) {
      error.value = unknownError instanceof Error ? unknownError.message : "Session data could not be read";
    } finally {
      loading.value = false;
    }
  }

  async function loadSession(sessionId: string) {
    loading.value = true;
    error.value = null;
    try {
      detail.value = await getSession(sessionId);
    } catch (unknownError) {
      error.value = unknownError instanceof Error ? unknownError.message : "Session detail could not be read";
    } finally {
      loading.value = false;
    }
  }

  return { sessions, detail, loading, error, loadSessions, loadSession };
});
