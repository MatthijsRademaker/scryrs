<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle, ConstellationGraph, EmptyState, EventTypeBar, FlameIndicator } from "@/shared/ui";
import { useHotspotStore } from "@/stores/hotspots";

const route = useRoute();
const hotspots = useHotspotStore();
const subjectKind = computed(() => String(route.params.subjectKind));
const subject = computed(() => String(route.params.subject));
const entry = computed(() => hotspots.entries.find((candidate) => candidate.subjectKind === subjectKind.value && candidate.subject === subject.value));
const eventBreakdown = computed(() => Object.entries(entry.value?.counts.eventType ?? {}));
const totalEvents = computed(() => eventBreakdown.value.reduce((sum, [, count]) => sum + count, 0));
const maxScore = computed(() => Math.max(1, ...hotspots.entries.map((candidate) => candidate.score)));
const constellationNodes = computed(() =>
  eventBreakdown.value.map(([type, count]) => ({ id: type, label: type, weight: count })),
);

onMounted(() => {
  if (!hotspots.report) void hotspots.load();
});
</script>

<template>
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-1">
      <h1 class="truncate text-2xl font-semibold tracking-tight">{{ subject }}</h1>
      <p class="text-sm text-muted-foreground">Subject drill-down for <span class="text-foreground/80">{{ subjectKind }}</span> hotspot evidence.</p>
    </header>

    <template v-if="entry">
      <!-- Glass stat tiles -->
      <section class="grid gap-4 md:grid-cols-3">
        <div class="glass-surface rounded-xl p-4">
          <div class="text-sm text-muted-foreground">Score</div>
          <div class="text-flame mt-1 text-2xl font-semibold tabular-nums">{{ entry.score }}</div>
          <div class="mt-2"><FlameIndicator :intensity="entry.score / maxScore" :show-icon="false" /></div>
        </div>
        <div class="glass-surface rounded-xl p-4">
          <div class="text-sm text-muted-foreground">Total Events</div>
          <div class="mt-1 text-2xl font-semibold tabular-nums">{{ totalEvents }}</div>
        </div>
        <div class="glass-surface rounded-xl p-4">
          <div class="text-sm text-muted-foreground">Sessions</div>
          <div class="mt-1 text-2xl font-semibold tabular-nums">{{ entry.sessionCount }}</div>
        </div>
      </section>

      <!-- Constellation of related event types -->
      <Card v-if="constellationNodes.length">
        <CardHeader><CardTitle>Evidence Constellation</CardTitle><CardDescription>Event types observed for this subject; node glow scales with volume.</CardDescription></CardHeader>
        <CardContent>
          <ConstellationGraph :nodes="constellationNodes" :center-label="subject" :height="260" />
        </CardContent>
      </Card>

      <Card>
        <CardHeader><CardTitle>Event Type Breakdown</CardTitle><CardDescription>Counts by trace event family.</CardDescription></CardHeader>
        <CardContent class="flex flex-col gap-4">
          <EventTypeBar :counts="entry.counts.eventType" legend :legend-limit="8" />
          <div class="flex flex-col gap-2">
            <div v-for="[eventType, count] in eventBreakdown" :key="eventType" class="grid grid-cols-[10rem_1fr_3rem] items-center gap-3 text-sm">
              <span class="truncate text-muted-foreground">{{ eventType }}</span>
              <div class="h-2 overflow-hidden rounded-full bg-muted/60"><div class="h-2 rounded-full bg-primary" :style="{ width: `${Math.max(6, (count / Math.max(totalEvents, 1)) * 100)}%` }" /></div>
              <span class="text-right tabular-nums">{{ count }}</span>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader><CardTitle>Activity Range</CardTitle><CardDescription>First and last observation for this subject.</CardDescription></CardHeader>
        <CardContent class="flex flex-col gap-3">
          <div class="rounded-xl border border-border bg-card/30 p-4">
            <div class="mb-2 flex flex-wrap justify-between gap-2 text-sm text-muted-foreground"><span>{{ entry.firstSeen }}</span><span>{{ entry.lastSeen }}</span></div>
            <div class="h-2 overflow-hidden rounded-full bg-muted/60"><div class="h-2 rounded-full bg-info shadow-[0_0_12px_-2px_var(--glow-accent)]" style="width: 100%" /></div>
          </div>
        </CardContent>
      </Card>
    </template>

    <Card v-else>
      <CardContent class="p-6"><EmptyState title="Subject not found" description="Hotspot report does not contain this subject." /></CardContent>
    </Card>
  </div>
</template>
