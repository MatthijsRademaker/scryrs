<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Alert, Badge, Button, Card, CardContent, CardDescription, CardHeader, CardTitle, EmptyState, EventTypeBar, SelectInput } from "@/shared/ui";
import { routeUnavailableMessage } from "@/shared/lib/dashboard-mode";
import { useEventStore } from "@/stores/events";
import { useSessionStore } from "@/stores/sessions";
import { useMetaStore } from "@/stores/meta";
import { colorForKey } from "@/shared/lib/viz";
import { formatSubject } from "@/shared/lib/subject";
import type { TraceEventItem } from "@/shared/api/client";

const events = useEventStore();
const sessions = useSessionStore();
const meta = useMetaStore();

function subjectDisplay(event: TraceEventItem) {
  return formatSubject(event.subject, meta.repositoryPath, event.subjectKind);
}
const selectedSession = ref("");
const options = computed(() => [{ label: "All sessions", value: "" }, ...sessions.sessions.map((session) => ({ label: session.sessionId, value: session.sessionId }))]);
const unavailableMessage = computed(() => routeUnavailableMessage("events", meta.mode));
const glowStart = ref(Number.POSITIVE_INFINITY);

function payloadPreview(payload: unknown) {
  return JSON.stringify(payload)?.slice(0, 220) ?? "null";
}
function loadMore() {
  glowStart.value = events.events.length;
  void events.load({ sessionId: selectedSession.value || null, cursor: events.nextCursor });
}

onMounted(async () => {
  await meta.ensureLoaded();
  if (!meta.isLiveMode) {
    void sessions.loadSessions();
    void events.load();
  }
});
watch(selectedSession, (sessionId) => {
  if (meta.isLiveMode) return;
  glowStart.value = Number.POSITIVE_INFINITY;
  void events.load({ sessionId: sessionId || null });
});
</script>
<template>
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-1">
      <h1 class="text-2xl font-semibold tracking-tight">Events</h1>
      <p class="text-sm text-muted-foreground">Live scrying feed — trace events color-coded by type.</p>
    </header>

    <Card v-if="unavailableMessage">
      <CardContent class="p-6"><EmptyState title="Unavailable in live mode" :description="unavailableMessage" /></CardContent>
    </Card>

    <Card v-else>
      <CardHeader class="gap-3">
        <div class="flex flex-col gap-1">
          <CardTitle>Feed</CardTitle>
          <CardDescription>Newest events first. Filter by session, then scry deeper.</CardDescription>
        </div>
        <div class="flex flex-wrap items-center gap-3">
          <SelectInput v-model="selectedSession" :options="options" />
          <Button v-if="selectedSession" variant="outline" @click="selectedSession = ''">Clear filter</Button>
        </div>
        <div v-if="Object.keys(events.distribution).length" class="pt-1">
          <EventTypeBar :counts="events.distribution" legend :legend-limit="6" />
        </div>
      </CardHeader>
      <CardContent class="flex flex-col gap-4">
        <Alert v-if="events.error" variant="destructive">{{ events.error }}</Alert>
        <EmptyState v-else-if="!events.loading && events.events.length === 0" title="No events" description="No trace events are available for this filter." />
        <template v-else>
          <ul class="flex max-h-[40rem] flex-col gap-1.5 overflow-auto rounded-xl border border-border bg-card/20 p-2 font-mono text-xs">
            <li
              v-for="(event, index) in events.events"
              :key="event.eventId"
              class="rounded-md border-l-2 bg-foreground/[0.02] px-3 py-2 transition-colors hover:bg-foreground/[0.05]"
              :class="index >= glowStart ? 'scry-glow' : 'scry-in'"
              :style="{ borderLeftColor: colorForKey(event.eventType), animationDelay: `${Math.min(index, 16) * 25}ms` }"
            >
              <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
                <span class="inline-flex items-center gap-1.5 font-medium" :style="{ color: colorForKey(event.eventType) }">
                  <span class="size-1.5 rounded-full" :style="{ backgroundColor: colorForKey(event.eventType) }"></span>
                  {{ event.eventType }}
                </span>
                <span class="text-muted-foreground">{{ event.timestamp }}</span>
                <span v-if="event.subject" class="inline-flex items-center gap-1.5 text-foreground/80" :title="subjectDisplay(event).full">
                  <Badge v-if="subjectDisplay(event).isExternal" variant="warning" class="shrink-0">EXTERNAL</Badge>
                  {{ subjectDisplay(event).label }}
                </span>
                <span v-else class="text-foreground/80">lifecycle</span>
                <span v-if="event.subjectKind" class="text-muted-foreground/70">{{ event.subjectKind }}</span>
              </div>
              <pre class="mt-1 overflow-hidden text-ellipsis whitespace-pre-wrap break-all text-[0.7rem] text-muted-foreground">{{ payloadPreview(event.payload) }}</pre>
            </li>
          </ul>
          <div v-if="events.nextCursor" class="flex justify-center">
            <Button variant="outline" :disabled="events.loading" @click="loadMore">{{ events.loading ? 'Loading…' : 'Load more' }}</Button>
          </div>
        </template>
      </CardContent>
    </Card>
  </div>
</template>
