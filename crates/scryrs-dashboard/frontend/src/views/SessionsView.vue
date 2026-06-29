<script setup lang="ts">
import { computed, onMounted } from "vue";
import { RouterLink } from "vue-router";
import { Alert, Badge, Card, CardContent, EmptyState, EventSparkline } from "@/shared/ui";
import { routeUnavailableMessage } from "@/shared/lib/dashboard-mode";
import { useSessionStore } from "@/stores/sessions";
import { useMetaStore } from "@/stores/meta";

const store = useSessionStore();
const meta = useMetaStore();
const maxEvents = computed(() => Math.max(1, ...store.sessions.map((session) => session.eventCount)));
const unavailableMessage = computed(() => routeUnavailableMessage("sessions", meta.mode));
onMounted(async () => {
  await meta.ensureLoaded();
  if (!meta.isLiveMode) {
    void store.loadSessions();
  }
});
function shortId(id: string) { return id.length > 16 ? `${id.slice(0, 16)}…` : id; }
</script>
<template>
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-1">
      <h1 class="text-2xl font-semibold tracking-tight">Sessions</h1>
      <p class="text-sm text-muted-foreground">Recent trace sessions ordered by start time. Active sessions pulse.</p>
    </header>

    <Card v-if="unavailableMessage">
      <CardContent class="p-6"><EmptyState title="Unavailable in live mode" :description="unavailableMessage" /></CardContent>
    </Card>

    <template v-else>
      <Alert v-if="store.error" variant="destructive">{{ store.error }}</Alert>
      <EmptyState v-else-if="!store.loading && store.sessions.length === 0" title="No sessions" description="Record trace events before opening the dashboard." />

      <div v-else class="flex flex-col gap-2.5">
        <Card
          v-for="(session, index) in store.sessions"
          :key="session.sessionId"
          class="scry-in flex items-center gap-4 p-4 transition-shadow duration-300 hover:shadow-[0_0_28px_-16px_var(--glow-accent)]"
          :style="{ animationDelay: `${Math.min(index, 12) * 40}ms` }"
        >
          <span class="relative flex size-2.5 shrink-0 items-center justify-center">
            <span
              class="size-2.5 rounded-full"
              :class="session.endedAt === null ? 'bg-primary pulse-dot shadow-[0_0_8px_2px_var(--glow-accent)]' : 'bg-muted-foreground/40'"
            ></span>
          </span>

          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <RouterLink
                :to="{ name: 'session-detail', params: { sessionId: session.sessionId } }"
                class="font-mono text-sm font-medium text-primary no-underline hover:underline"
                :title="session.sessionId"
              >
                {{ shortId(session.sessionId) }}
              </RouterLink>
              <Badge v-if="session.endedAt === null" variant="default" class="text-[0.65rem]">Active</Badge>
            </div>
            <div class="mt-0.5 truncate text-xs text-muted-foreground">
              started {{ session.startedAt }} · {{ session.source }}
            </div>
          </div>

          <div class="flex items-center gap-3">
            <div class="text-right">
              <div class="tabular-nums text-sm font-medium text-foreground">{{ session.eventCount }}</div>
              <div class="text-[0.65rem] uppercase tracking-wider text-muted-foreground">events</div>
            </div>
            <EventSparkline :values="[session.eventCount]" :max="maxEvents" />
          </div>
        </Card>
      </div>
    </template>
  </div>
</template>
