<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Alert, Button, Card, CardContent, CardDescription, CardHeader, CardTitle, EmptyState, SelectInput } from "@/shared/ui";
import { useEventStore } from "@/stores/events";
import { useSessionStore } from "@/stores/sessions";

const events = useEventStore();
const sessions = useSessionStore();
const selectedSession = ref("");
const options = computed(() => [{ label: "All sessions", value: "" }, ...sessions.sessions.map((session) => ({ label: session.sessionId, value: session.sessionId }))]);
const maxCount = computed(() => Math.max(...Object.values(events.distribution), 1));

onMounted(() => { void sessions.loadSessions(); void events.load(); });
watch(selectedSession, (sessionId) => { void events.load({ sessionId: sessionId || null }); });
</script>
<template>
  <Card>
    <CardHeader><CardTitle>Event Distribution</CardTitle><CardDescription>Aggregate trace events grouped by event_type.</CardDescription></CardHeader>
    <CardContent class="flex flex-col gap-4">
      <div class="flex flex-wrap items-center gap-3"><SelectInput v-model="selectedSession" :options="options" /><Button v-if="selectedSession" variant="outline" @click="selectedSession = ''">Clear filter</Button></div>
      <Alert v-if="events.error" variant="destructive">{{ events.error }}</Alert>
      <EmptyState v-else-if="!events.loading && Object.keys(events.distribution).length === 0" title="No events" description="No trace events are available for this filter." />
      <div v-else class="flex flex-col gap-3">
        <div v-for="[eventType, count] in Object.entries(events.distribution)" :key="eventType" class="grid grid-cols-[12rem_1fr_4rem] items-center gap-3">
          <span class="font-medium">{{ eventType }}</span>
          <div class="h-4 rounded-full bg-muted"><div class="h-4 rounded-full bg-success" :style="{ width: `${Math.max(8, (count / maxCount) * 100)}%` }" /></div>
          <span class="text-right tabular-nums">{{ count }}</span>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
