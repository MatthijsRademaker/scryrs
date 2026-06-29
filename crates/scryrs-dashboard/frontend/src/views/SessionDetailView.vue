<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { Badge, Card, CardContent, CardDescription, CardHeader, CardTitle, ConstellationGraph, EmptyState, EventSparkline } from "@/shared/ui";
import { routeUnavailableMessage } from "@/shared/lib/dashboard-mode";
import { useSessionStore } from "@/stores/sessions";
import { useMetaStore } from "@/stores/meta";
import { colorForKey } from "@/shared/lib/viz";
import { formatSubject } from "@/shared/lib/subject";
import type { TraceEventItem } from "@/shared/api/client";

const route = useRoute();
const store = useSessionStore();
const meta = useMetaStore();

function subjectDisplay(event: TraceEventItem) {
  return formatSubject(event.subject, meta.repositoryPath, event.subjectKind);
}
const sessionId = computed(() => String(route.params.sessionId));
const shortId = computed(() => (sessionId.value.length > 18 ? `${sessionId.value.slice(0, 18)}…` : sessionId.value));
const unavailableMessage = computed(() => routeUnavailableMessage("session-detail", meta.mode));

const eventTypeCounts = computed(() => {
  const counts: Record<string, number> = {};
  for (const event of store.detail?.events ?? []) {
    counts[event.eventType] = (counts[event.eventType] ?? 0) + 1;
  }
  return counts;
});
const constellationNodes = computed(() =>
  Object.entries(eventTypeCounts.value).map(([type, count]) => ({ id: type, label: type, weight: count })),
);

const sparkValues = computed(() => {
  const times = (store.detail?.events ?? [])
    .map((event) => Date.parse(event.timestamp))
    .filter((t) => !Number.isNaN(t));
  if (times.length === 0) return [store.detail?.events.length ?? 0];
  const min = Math.min(...times);
  const max = Math.max(...times);
  if (max === min) return [times.length];
  const bins = Math.min(16, Math.max(4, times.length));
  const out = new Array(bins).fill(0);
  for (const t of times) {
    const idx = Math.min(bins - 1, Math.floor(((t - min) / (max - min)) * bins));
    out[idx] += 1;
  }
  return out;
});

onMounted(async () => {
  await meta.ensureLoaded();
  if (!meta.isLiveMode) {
    void store.loadSession(sessionId.value);
  }
});
function payloadPreview(payload: unknown) { return JSON.stringify(payload)?.slice(0, 200) ?? "null"; }
</script>
<template>
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-1">
      <h1 class="truncate font-mono text-2xl font-semibold tracking-tight" :title="sessionId">{{ shortId }}</h1>
      <p class="text-sm text-muted-foreground">Event timeline and payload preview.</p>
    </header>

    <Card v-if="unavailableMessage">
      <CardContent class="p-6"><EmptyState title="Unavailable in live mode" :description="unavailableMessage" /></CardContent>
    </Card>

    <Card v-else-if="store.error">
      <CardContent class="p-6"><EmptyState title="Session unavailable" :description="store.error" /></CardContent>
    </Card>

    <template v-else-if="store.detail">
      <section class="grid gap-4 md:grid-cols-3">
        <div class="glass-surface rounded-xl p-4">
          <div class="text-sm text-muted-foreground">Started</div>
          <div class="mt-1 text-sm font-medium">{{ store.detail.session.startedAt }}</div>
        </div>
        <div class="glass-surface rounded-xl p-4">
          <div class="flex items-center gap-2 text-sm text-muted-foreground">
            Ended
            <span v-if="store.detail.session.endedAt === null" class="inline-flex items-center gap-1.5 text-primary">
              <span class="size-1.5 rounded-full bg-primary pulse-dot"></span>active
            </span>
          </div>
          <div class="mt-1 text-sm font-medium">{{ store.detail.session.endedAt ?? '—' }}</div>
        </div>
        <div class="glass-surface rounded-xl p-4">
          <div class="text-sm text-muted-foreground">Events</div>
          <div class="mt-1 flex items-center justify-between gap-2">
            <span class="text-2xl font-semibold tabular-nums">{{ store.detail.session.eventCount }}</span>
            <EventSparkline :values="sparkValues" :width="120" :height="32" />
          </div>
        </div>
      </section>

      <Card v-if="constellationNodes.length">
        <CardHeader><CardTitle>Event Constellation</CardTitle><CardDescription>Event types captured in this session; node glow scales with volume.</CardDescription></CardHeader>
        <CardContent>
          <ConstellationGraph :nodes="constellationNodes" :center-label="shortId" :height="260" />
        </CardContent>
      </Card>

      <Card>
        <CardHeader><CardTitle>Events</CardTitle><CardDescription>Raw trace events captured for this session.</CardDescription></CardHeader>
        <CardContent>
          <ul class="flex max-h-[36rem] flex-col gap-1.5 overflow-auto rounded-xl border border-border bg-card/20 p-2 font-mono text-xs">
            <li
              v-for="event in store.detail.events"
              :key="event.eventId"
              class="rounded-md border-l-2 bg-foreground/[0.02] px-3 py-2"
              :style="{ borderLeftColor: colorForKey(event.eventType) }"
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
              </div>
              <pre class="mt-1 overflow-hidden whitespace-pre-wrap break-all text-[0.7rem] text-muted-foreground">{{ payloadPreview(event.payload) }}</pre>
            </li>
          </ul>
        </CardContent>
      </Card>
    </template>
  </div>
</template>
