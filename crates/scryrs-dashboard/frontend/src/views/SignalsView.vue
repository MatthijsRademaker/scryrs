<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import { Alert, Badge, Button, Card, CardContent, CardDescription, CardHeader, CardTitle, EmptyState, Table } from "@/shared/ui";
import { routeUnavailableMessage } from "@/shared/lib/dashboard-mode";
import { useMetaStore } from "@/stores/meta";
import { useSignalStore } from "@/stores/signals";

const meta = useMetaStore();
const signals = useSignalStore();
const unavailableMessage = computed(() => routeUnavailableMessage("signals", meta.mode));

const connectionVariant = computed(() => {
  switch (signals.connectionState) {
    case "connected":
      return "success";
    case "connecting":
    case "reconnecting":
      return "info";
    case "error":
      return "destructive";
    default:
      return "outline";
  }
});

function restart() {
  signals.stop();
  signals.start();
}

onMounted(async () => {
  await meta.ensureLoaded();
  if (meta.isLiveMode) {
    signals.start();
  }
});

onUnmounted(() => {
  signals.stop();
});
</script>

<template>
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-2">
      <div class="flex flex-wrap items-center gap-3">
        <h1 class="text-2xl font-semibold tracking-tight">Signals</h1>
        <Badge :variant="connectionVariant">{{ signals.connectionState }}</Badge>
      </div>
      <p class="text-sm text-muted-foreground">Replay persisted hotspot signals, then tail new live signals without losing the last seen cursor.</p>
    </header>

    <Card v-if="unavailableMessage">
      <CardContent class="p-6"><EmptyState title="Signals unavailable" :description="unavailableMessage" /></CardContent>
    </Card>

    <template v-else>
      <Alert v-if="signals.error" variant="destructive">
        <div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <span>{{ signals.error }}</span>
          <Button variant="outline" @click="restart">Reconnect</Button>
        </div>
      </Alert>

      <Card>
        <CardHeader class="gap-3">
          <div class="flex flex-col gap-1">
            <CardTitle>Hotspot signal timeline</CardTitle>
            <CardDescription>{{ meta.repositoryId ? `Repository ${meta.repositoryId}` : 'Live repository stream' }} · last seen signal {{ signals.lastSeenId }}</CardDescription>
          </div>
        </CardHeader>
        <CardContent>
          <EmptyState v-if="signals.signals.length === 0" title="No signals yet" description="Waiting for replayed or live hotspot signals from the configured server." />
          <Table v-else>
            <thead class="bg-card/40 text-xs uppercase tracking-wider text-muted-foreground">
              <tr>
                <th class="px-3 py-2.5 text-left font-medium">ID</th>
                <th class="px-3 py-2.5 text-left font-medium">Subject</th>
                <th class="px-3 py-2.5 text-left font-medium">Kind</th>
                <th class="px-3 py-2.5 text-left font-medium">Score</th>
                <th class="px-3 py-2.5 text-left font-medium">Threshold / Delta</th>
                <th class="px-3 py-2.5 text-left font-medium">Timestamp</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="signal in signals.signals" :key="signal.id" class="border-t border-border transition-colors hover:bg-foreground/[0.03]">
                <td class="px-3 py-2.5 font-mono tabular-nums text-muted-foreground">{{ signal.id }}</td>
                <td class="px-3 py-2.5 font-medium text-foreground">{{ signal.subject }}</td>
                <td class="px-3 py-2.5 text-muted-foreground">{{ signal.subjectKind }}</td>
                <td class="px-3 py-2.5 tabular-nums">{{ signal.score }}</td>
                <td class="px-3 py-2.5 text-sm text-muted-foreground">threshold {{ signal.threshold }} · Δ {{ signal.delta }}</td>
                <td class="px-3 py-2.5 text-muted-foreground">{{ signal.createdAt }}</td>
              </tr>
            </tbody>
          </Table>
        </CardContent>
      </Card>
    </template>
  </div>
</template>
