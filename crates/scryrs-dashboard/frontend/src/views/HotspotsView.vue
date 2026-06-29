<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { RouterLink } from "vue-router";
import {
  Alert,
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  EmptyState,
  EventTypeBar,
  FlameIndicator,
  OutcomePulse,
  Table,
} from "@/shared/ui";
import { useHotspotStore } from "@/stores/hotspots";
import { useMetaStore } from "@/stores/meta";
import { hotspotSubjectDisplay } from "@/shared/lib/dashboard-mode";
import type { HotspotEntry } from "@/shared/api/client";

const store = useHotspotStore();
const meta = useMetaStore();

function subjectDisplay(entry: HotspotEntry) {
  return hotspotSubjectDisplay(entry, {
    mode: meta.mode,
    repositoryPath: meta.repositoryPath,
  });
}
const sortKey = ref<keyof HotspotEntry>("rank");
const sortAsc = ref(true);
const columns: { key: keyof HotspotEntry; label: string }[] = [
  { key: "rank", label: "Rank" },
  { key: "subject", label: "Subject" },
  { key: "score", label: "Score" },
  { key: "sessionCount", label: "Session Count" },
  { key: "firstSeen", label: "First Seen" },
  { key: "lastSeen", label: "Last Seen" },
];
const sortedEntries = computed(() => [...store.entries].sort((a, b) => {
  const left = a[sortKey.value];
  const right = b[sortKey.value];
  const result = typeof left === "number" && typeof right === "number" ? left - right : String(left).localeCompare(String(right));
  return sortAsc.value ? result : -result;
}));
const headerDescription = computed(() => meta.isLiveMode
  ? "Current server-ranked subjects. Flame intensity tracks relative cumulative heat."
  : "Ranked subjects from .scryrs/hotspots.json — flame intensity tracks relative heat.");
const emptyDescription = computed(() => meta.isLiveMode
  ? "No live hotspots are available for this repository yet."
  : "Run scryrs hotspots . to materialize .scryrs/hotspots.json.");

function sortBy(key: keyof HotspotEntry) {
  if (sortKey.value === key) sortAsc.value = !sortAsc.value;
  else {
    sortKey.value = key;
    sortAsc.value = true;
  }
}

const maxScore = computed(() => Math.max(1, ...store.entries.map((entry) => entry.score)));
const topEntries = computed(() => [...store.entries].sort((a, b) => a.rank - b.rank).slice(0, 3));
function totalEvents(entry: HotspotEntry) {
  return Object.values(entry.counts.eventType).reduce((sum, count) => sum + count, 0);
}

onMounted(() => { void store.load(); void meta.ensureLoaded(); });
</script>

<template>
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-1">
      <h1 class="text-2xl font-semibold tracking-tight">Hotspots</h1>
      <p class="text-sm text-muted-foreground">{{ headerDescription }}</p>
    </header>

    <Alert v-if="store.error" variant="destructive">
      <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <span>{{ store.error }}</span>
        <Button variant="outline" @click="store.load">Retry</Button>
      </div>
    </Alert>

    <EmptyState v-else-if="!store.loading && store.entries.length === 0" title="No hotspot data" :description="emptyDescription" />

    <template v-else>
      <section v-if="topEntries.length" class="grid gap-4 md:grid-cols-3">
        <Card
          v-for="(entry, index) in topEntries"
          :key="`${entry.subjectKind}:${entry.subject}`"
          class="scry-in flex flex-col gap-3 p-5 transition-shadow duration-300 hover:shadow-[0_0_34px_-14px_var(--glow-hot)]"
          :style="{ animationDelay: `${index * 70}ms` }"
        >
          <div class="flex items-start justify-between gap-2">
            <Badge variant="outline" class="font-mono">#{{ entry.rank }}</Badge>
            <span class="truncate text-xs uppercase tracking-wider text-muted-foreground">{{ entry.subjectKind }}</span>
          </div>
          <RouterLink
            :to="{ name: 'subject-detail', params: { subjectKind: entry.subjectKind, subject: entry.subject } }"
            class="flex items-center gap-1.5 truncate text-lg font-semibold text-foreground no-underline transition-colors hover:text-primary"
            :title="entry.subject"
          >
            <Badge v-if="subjectDisplay(entry).isExternal" variant="warning" class="shrink-0">EXTERNAL</Badge>
            <span class="truncate">{{ subjectDisplay(entry).label }}</span>
          </RouterLink>
          <div class="flex items-end justify-between gap-3">
            <span class="text-flame text-3xl font-semibold tabular-nums">{{ entry.score }}</span>
            <div class="flex-1 pb-2"><FlameIndicator :intensity="entry.score / maxScore" /></div>
          </div>
          <EventTypeBar :counts="entry.counts.eventType" legend />
          <OutcomePulse :counts="entry.counts.outcome" />
          <div class="mt-1 flex items-center justify-between border-t border-border pt-3 text-xs text-muted-foreground">
            <span><span class="tabular-nums text-foreground/80">{{ entry.sessionCount }}</span> sessions</span>
            <span><span class="tabular-nums text-foreground/80">{{ totalEvents(entry) }}</span> events</span>
          </div>
        </Card>
      </section>

      <Card>
        <CardHeader>
          <CardTitle>All subjects</CardTitle>
          <CardDescription>{{ meta.isLiveMode ? 'Server-ranked cumulative hotspots. Click a column to sort.' : 'Full ranked breakdown. Click a column to sort.' }}</CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <thead class="bg-card/40 text-xs uppercase tracking-wider text-muted-foreground">
              <tr>
                <th v-for="column in columns" :key="column.key" class="px-3 py-2.5 text-left font-medium">
                  <button class="inline-flex items-center gap-1 transition-colors hover:text-foreground" @click="sortBy(column.key)">
                    {{ column.label }}
                    <span v-if="sortKey === column.key" class="text-primary">{{ sortAsc ? '▲' : '▼' }}</span>
                  </button>
                </th>
                <th class="px-3 py-2.5 text-left font-medium">Total Events</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="entry in sortedEntries" :key="`${entry.subjectKind}:${entry.subject}`" class="border-t border-border transition-colors hover:bg-foreground/[0.03]">
                <td class="px-3 py-2.5 font-mono tabular-nums text-muted-foreground">{{ entry.rank }}</td>
                <td class="px-3 py-2.5">
                  <RouterLink :to="{ name: 'subject-detail', params: { subjectKind: entry.subjectKind, subject: entry.subject } }" class="inline-flex items-center gap-1.5 font-medium text-primary no-underline hover:underline" :title="entry.subject">
                    <Badge v-if="subjectDisplay(entry).isExternal" variant="warning" class="shrink-0">EXTERNAL</Badge>
                    <span>{{ subjectDisplay(entry).label }}</span>
                  </RouterLink>
                  <div class="text-xs text-muted-foreground">{{ entry.subjectKind }}</div>
                  <div class="mt-1.5 max-w-44"><EventTypeBar :counts="entry.counts.eventType" /></div>
                </td>
                <td class="px-3 py-2.5 tabular-nums">{{ entry.score }}</td>
                <td class="px-3 py-2.5 tabular-nums">{{ entry.sessionCount }}</td>
                <td class="px-3 py-2.5 text-muted-foreground">{{ entry.firstSeen }}</td>
                <td class="px-3 py-2.5 text-muted-foreground">{{ entry.lastSeen }}</td>
                <td class="px-3 py-2.5 tabular-nums">{{ totalEvents(entry) }}</td>
              </tr>
            </tbody>
          </Table>
        </CardContent>
      </Card>
    </template>
  </div>
</template>
