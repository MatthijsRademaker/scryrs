<script setup lang="ts">
import { computed, onMounted } from "vue";
import { RouterLink } from "vue-router";
import { Alert, Badge, Card, CardContent, EmptyState } from "@/shared/ui";
import { routeUnavailableMessage } from "@/shared/lib/dashboard-mode";
import { useProposalStore } from "@/stores/proposals";
import { useMetaStore } from "@/stores/meta";

const store = useProposalStore();
const meta = useMetaStore();

const unavailableMessage = computed(() =>
  routeUnavailableMessage("proposals", meta.mode),
);

onMounted(async () => {
  await meta.ensureLoaded();
  if (!meta.isLiveMode) {
    void store.loadProposals();
  }
});

function shortId(id: string) {
  return id.length > 16 ? `${id.slice(0, 16)}…` : id;
}

function stateVariant(
  state: string,
): "default" | "secondary" | "outline" | "destructive" {
  switch (state) {
    case "accepted":
      return "default";
    case "rejected":
      return "destructive";
    default:
      return "secondary";
  }
}
</script>

<template>
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-1">
      <h1 class="text-2xl font-semibold tracking-tight">Proposals</h1>
      <p class="text-sm text-muted-foreground">
        Reviewable knowledge proposals from the proposal inbox.
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
        v-else-if="!store.loading && store.rows.length === 0"
        title="No proposals"
        description="Run `scryrs propose` to generate proposals before opening the dashboard."
      />

      <div v-else class="flex flex-col gap-2.5">
        <Card
          v-for="row in store.rows"
          :key="row.proposalId"
          class="scry-in flex items-center gap-4 p-4 transition-shadow duration-300 hover:shadow-[0_0_28px_-16px_var(--glow-accent)]"
        >
          <span
            class="relative flex size-2.5 shrink-0 items-center justify-center"
          >
            <span
              class="size-2.5 rounded-full"
              :class="
                row.state === 'pending'
                  ? 'bg-muted-foreground/40'
                  : row.state === 'accepted'
                    ? 'bg-primary shadow-[0_0_8px_2px_var(--glow-accent)]'
                    : 'bg-destructive'
              "
            ></span>
          </span>

          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <RouterLink
                :to="{
                  name: 'proposal-detail',
                  params: { proposalId: row.proposalId },
                }"
                class="font-mono text-sm font-medium text-primary no-underline hover:underline"
                :title="row.proposalId"
              >
                {{ shortId(row.proposalId) }}
              </RouterLink>
              <Badge :variant="stateVariant(row.state)" class="text-[0.65rem]">
                {{ row.state }}
              </Badge>
            </div>
            <div class="mt-0.5 truncate text-sm font-medium">
              {{ row.title }}
            </div>
            <div class="truncate text-xs text-muted-foreground">
              {{ row.targetType }} · {{ row.createdAt }}
            </div>
          </div>
        </Card>
      </div>
    </template>
  </div>
</template>
