<script setup lang="ts">
import { computed, onMounted } from "vue";
import { RouterLink } from "vue-router";
import { Alert, Badge, Card, CardContent, EmptyState } from "@/shared/ui";
import { routeUnavailableMessage } from "@/shared/lib/dashboard-mode";
import { useAcceptedStore } from "@/stores/accepted";
import { useMetaStore } from "@/stores/meta";
import type { PublishSurfaceStatus } from "@/shared/api/client";

const store = useAcceptedStore();
const meta = useMetaStore();

const unavailableMessage = computed(() =>
  routeUnavailableMessage("accepted", meta.mode),
);

onMounted(async () => {
  await meta.ensureLoaded();
  if (!meta.isLiveMode) {
    void store.fetchList();
  }
});

function shortId(id: string) {
  return id.length > 16 ? `${id.slice(0, 16)}…` : id;
}

function publishBadgeVariant(
  status: PublishSurfaceStatus["status"],
): "default" | "secondary" | "outline" | "destructive" {
  switch (status) {
    case "published":
      return "default";
    case "not_published":
      return "secondary";
    case "not_publishable":
      return "outline";
    case "unknown":
      return "secondary";
  }
}

function publishBadgeLabel(surface: string, status: string): string {
  const surfaceLabel = surface === "rspress" ? "Rspress" : "Markdown";
  return `${surfaceLabel}: ${status.replaceAll("_", " ")}`;
}
</script>

<template>
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-1">
      <h1 class="text-2xl font-semibold tracking-tight">Accepted Knowledge</h1>
      <p class="text-sm text-muted-foreground">
        Accepted review decisions with publish-status visibility.
      </p>
    </header>

    <Card v-if="unavailableMessage">
      <CardContent class="p-6">
        <EmptyState
          title="Unavailable in live mode"
          :description="unavailableMessage"
        />
      </CardContent>
    </Card>

    <template v-else>
      <Alert v-if="store.error" variant="destructive">{{
        store.error
      }}</Alert>

      <EmptyState
        v-else-if="!store.loading && store.items.length === 0"
        title="No accepted knowledge"
        description="Run `scryrs proposals accept` to accept proposals before opening the dashboard."
      />

      <div v-else-if="store.loading" class="flex items-center gap-2 text-sm text-muted-foreground">
        <span>Loading accepted knowledge…</span>
      </div>

      <div v-else class="flex flex-col gap-2.5">
        <Card
          v-for="row in store.items"
          :key="row.proposalId"
          class="flex items-center gap-4 p-4 transition-shadow duration-300 hover:shadow-[0_0_28px_-16px_var(--glow-accent)]"
        >
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <RouterLink
                :to="{
                  name: 'accepted-detail',
                  params: { proposalId: row.proposalId },
                }"
                class="font-mono text-sm font-medium text-primary no-underline hover:underline"
                :title="row.proposalId"
              >
                {{ shortId(row.proposalId) }}
              </RouterLink>
            </div>
            <div class="mt-0.5 truncate text-sm font-medium">
              {{ row.title }}
            </div>
            <div class="flex flex-wrap items-center gap-1.5 truncate text-xs text-muted-foreground">
              <span>{{ row.targetType }}</span>
              <span>·</span>
              <span>{{ row.reviewer }}</span>
              <span>·</span>
              <span>{{ row.decidedAt }}</span>
            </div>
            <div class="mt-1 flex flex-wrap gap-1">
              <Badge
                v-for="(ps, idx) in row.publishStatus"
                :key="idx"
                :variant="publishBadgeVariant(ps.status)"
                class="text-[0.6rem]"
              >
                {{ publishBadgeLabel(ps.surface, ps.status) }}
              </Badge>
            </div>
          </div>
        </Card>
      </div>
    </template>
  </div>
</template>
