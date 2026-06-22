<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { Badge, Card, CardContent, CardDescription, CardHeader, CardTitle, EmptyState } from "@/shared/ui";
import { useSessionStore } from "@/stores/sessions";

const route = useRoute();
const store = useSessionStore();
const sessionId = computed(() => String(route.params.sessionId));
onMounted(() => { void store.loadSession(sessionId.value); });
function payloadPreview(payload: unknown) { return JSON.stringify(payload)?.slice(0, 160) ?? "null"; }
</script>
<template>
  <div class="flex flex-col gap-6">
    <Card>
      <CardHeader><CardTitle>Session {{ sessionId }}</CardTitle><CardDescription>Scrollable event timeline and payload preview.</CardDescription></CardHeader>
      <CardContent v-if="store.error"><EmptyState title="Session unavailable" :description="store.error" /></CardContent>
      <CardContent v-else-if="store.detail" class="grid gap-4 md:grid-cols-3">
        <div class="rounded-lg border bg-muted p-4"><div class="text-sm text-muted-foreground">Started</div><div class="font-medium">{{ store.detail.session.startedAt }}</div></div>
        <div class="rounded-lg border bg-muted p-4"><div class="text-sm text-muted-foreground">Ended</div><div class="font-medium">{{ store.detail.session.endedAt ?? 'Active' }}</div></div>
        <div class="rounded-lg border bg-muted p-4"><div class="text-sm text-muted-foreground">Events</div><div class="font-medium">{{ store.detail.session.eventCount }}</div></div>
      </CardContent>
    </Card>
    <Card v-if="store.detail">
      <CardHeader><CardTitle>Events</CardTitle><CardDescription>Raw trace events captured for this session.</CardDescription></CardHeader>
      <CardContent><div class="max-h-[36rem] overflow-auto rounded-lg border">
        <div v-for="event in store.detail.events" :key="event.eventId" class="border-b p-4 last:border-b-0">
          <div class="flex flex-wrap items-center gap-2"><Badge>{{ event.eventType }}</Badge><span class="text-sm text-muted-foreground">{{ event.timestamp }}</span><span class="text-sm">{{ event.subject ?? 'lifecycle' }}</span></div>
          <pre class="mt-2 overflow-auto rounded-md bg-muted p-3 text-xs">{{ payloadPreview(event.payload) }}</pre>
        </div>
      </div></CardContent>
    </Card>
  </div>
</template>
