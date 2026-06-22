<script setup lang="ts">
import { onMounted } from "vue";
import { RouterLink } from "vue-router";
import { Alert, Card, CardContent, CardDescription, CardHeader, CardTitle, EmptyState, Table } from "@/shared/ui";
import { useSessionStore } from "@/stores/sessions";

const store = useSessionStore();
onMounted(() => { void store.loadSessions(); });
function shortId(id: string) { return id.length > 12 ? `${id.slice(0, 12)}…` : id; }
</script>
<template>
  <Card>
    <CardHeader><CardTitle>Sessions</CardTitle><CardDescription>Recent trace sessions ordered by start time.</CardDescription></CardHeader>
    <CardContent class="flex flex-col gap-4">
      <Alert v-if="store.error" variant="destructive">{{ store.error }}</Alert>
      <EmptyState v-else-if="!store.loading && store.sessions.length === 0" title="No sessions" description="Record trace events before opening the dashboard." />
      <Table v-else>
        <thead class="bg-muted/60"><tr><th class="px-3 py-2 text-left">Session</th><th class="px-3 py-2 text-left">Started</th><th class="px-3 py-2 text-left">Ended</th><th class="px-3 py-2 text-left">Events</th><th class="px-3 py-2 text-left">Source</th></tr></thead>
        <tbody>
          <tr v-for="session in store.sessions" :key="session.sessionId" class="border-t">
            <td class="px-3 py-2 font-medium"><RouterLink :to="{ name: 'session-detail', params: { sessionId: session.sessionId } }" class="text-primary no-underline hover:underline">{{ shortId(session.sessionId) }}</RouterLink></td>
            <td class="px-3 py-2">{{ session.startedAt }}</td>
            <td class="px-3 py-2">{{ session.endedAt ?? 'Active' }}</td>
            <td class="px-3 py-2">{{ session.eventCount }}</td>
            <td class="px-3 py-2">{{ session.source }}</td>
          </tr>
        </tbody>
      </Table>
    </CardContent>
  </Card>
</template>
