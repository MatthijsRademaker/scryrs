<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { RouterLink } from "vue-router";
import { Alert, Button, Card, CardContent, CardDescription, CardHeader, CardTitle, EmptyState, Table } from "@/shared/ui";
import { useHotspotStore } from "@/stores/hotspots";
import type { HotspotEntry } from "@/shared/api/client";

const store = useHotspotStore();
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

function sortBy(key: keyof HotspotEntry) {
  if (sortKey.value === key) sortAsc.value = !sortAsc.value;
  else {
    sortKey.value = key;
    sortAsc.value = true;
  }
}

onMounted(() => { void store.load(); });
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle>Hotspots</CardTitle>
      <CardDescription>Ranked subjects from .scryrs/hotspots.json.</CardDescription>
    </CardHeader>
    <CardContent class="flex flex-col gap-4">
      <Alert v-if="store.error" variant="destructive">
        <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <span>{{ store.error }}</span>
          <Button variant="outline" @click="store.load">Retry</Button>
        </div>
      </Alert>
      <EmptyState v-else-if="!store.loading && sortedEntries.length === 0" title="No hotspot data" description="Run scryrs hotspots . to materialize .scryrs/hotspots.json." />
      <Table v-else>
        <thead class="bg-muted/60">
          <tr>
            <th v-for="column in columns" :key="column.key" class="px-3 py-2 text-left font-medium">
              <button class="inline-flex items-center gap-1" @click="sortBy(column.key)">{{ column.label }}</button>
            </th>
            <th class="px-3 py-2 text-left font-medium">Total Events</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="entry in sortedEntries" :key="`${entry.subjectKind}:${entry.subject}`" class="border-t">
            <td class="px-3 py-2">{{ entry.rank }}</td>
            <td class="px-3 py-2 font-medium">
              <RouterLink :to="{ name: 'subject-detail', params: { subjectKind: entry.subjectKind, subject: entry.subject } }" class="text-primary no-underline hover:underline">{{ entry.subject }}</RouterLink>
              <div class="text-xs text-muted-foreground">{{ entry.subjectKind }}</div>
            </td>
            <td class="px-3 py-2">{{ entry.score }}</td>
            <td class="px-3 py-2">{{ entry.sessionCount }}</td>
            <td class="px-3 py-2">{{ Object.values(entry.counts.eventType).reduce((sum, count) => sum + count, 0) }}</td>
            <td class="px-3 py-2">{{ entry.firstSeen }}</td>
            <td class="px-3 py-2">{{ entry.lastSeen }}</td>
          </tr>
        </tbody>
      </Table>
    </CardContent>
  </Card>
</template>
