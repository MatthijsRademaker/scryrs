<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { Badge, Card, CardContent, CardDescription, CardHeader, CardTitle, EmptyState } from "@/shared/ui";
import { useHotspotStore } from "@/stores/hotspots";

const route = useRoute();
const hotspots = useHotspotStore();
const subjectKind = computed(() => String(route.params.subjectKind));
const subject = computed(() => String(route.params.subject));
const entry = computed(() => hotspots.entries.find((candidate) => candidate.subjectKind === subjectKind.value && candidate.subject === subject.value));
const eventBreakdown = computed(() => Object.entries(entry.value?.counts.eventType ?? {}));
const totalEvents = computed(() => eventBreakdown.value.reduce((sum, [, count]) => sum + count, 0));

onMounted(() => {
  if (!hotspots.report) void hotspots.load();
});
</script>

<template>
  <div class="flex flex-col gap-6">
    <Card>
      <CardHeader>
        <CardTitle>{{ subject }}</CardTitle>
        <CardDescription>Subject drill-down for {{ subjectKind }} hotspot evidence.</CardDescription>
      </CardHeader>
      <CardContent v-if="entry" class="grid gap-4 md:grid-cols-3">
        <div class="rounded-lg border bg-muted p-4"><div class="text-sm text-muted-foreground">Score</div><div class="text-2xl font-semibold">{{ entry.score }}</div></div>
        <div class="rounded-lg border bg-muted p-4"><div class="text-sm text-muted-foreground">Total Events</div><div class="text-2xl font-semibold">{{ totalEvents }}</div></div>
        <div class="rounded-lg border bg-muted p-4"><div class="text-sm text-muted-foreground">Sessions</div><div class="text-2xl font-semibold">{{ entry.sessionCount }}</div></div>
      </CardContent>
      <CardContent v-else><EmptyState title="Subject not found" description="Hotspot report does not contain this subject." /></CardContent>
    </Card>

    <Card v-if="entry">
      <CardHeader><CardTitle>Event Type Breakdown</CardTitle><CardDescription>Counts by trace event family.</CardDescription></CardHeader>
      <CardContent class="flex flex-col gap-3">
        <div v-for="[eventType, count] in eventBreakdown" :key="eventType" class="grid grid-cols-[10rem_1fr_4rem] items-center gap-3">
          <Badge variant="secondary">{{ eventType }}</Badge>
          <div class="h-3 rounded-full bg-muted"><div class="h-3 rounded-full bg-primary" :style="{ width: `${Math.max(8, (count / Math.max(totalEvents, 1)) * 100)}%` }" /></div>
          <span class="text-right text-sm tabular-nums">{{ count }}</span>
        </div>
      </CardContent>
    </Card>

    <Card v-if="entry">
      <CardHeader><CardTitle>Session Timeline</CardTitle><CardDescription>Semantic timeline placeholder from hotspot first/last seen range.</CardDescription></CardHeader>
      <CardContent class="flex flex-col gap-3">
        <div class="rounded-lg border p-4">
          <div class="mb-2 flex justify-between text-sm text-muted-foreground"><span>{{ entry.firstSeen }}</span><span>{{ entry.lastSeen }}</span></div>
          <div class="h-4 rounded-full bg-info-soft"><div class="h-4 rounded-full bg-info" :style="{ width: '100%' }" /></div>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
