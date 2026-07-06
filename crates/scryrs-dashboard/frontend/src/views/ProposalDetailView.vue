<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { Alert, Badge, Card, CardContent, CardHeader, CardTitle, CardDescription, EmptyState } from "@/shared/ui";
import { routeUnavailableMessage } from "@/shared/lib/dashboard-mode";
import { useProposalStore } from "@/stores/proposals";
import { useMetaStore } from "@/stores/meta";

const route = useRoute();
const store = useProposalStore();
const meta = useMetaStore();

const proposalId = computed(() => String(route.params.proposalId));
const shortId = computed(() =>
  proposalId.value.length > 18
    ? `${proposalId.value.slice(0, 18)}…`
    : proposalId.value,
);
const unavailableMessage = computed(() =>
  routeUnavailableMessage("proposal-detail", meta.mode),
);

onMounted(async () => {
  await meta.ensureLoaded();
  if (!meta.isLiveMode) {
    void store.loadProposal(proposalId.value);
  }
});

function contentDisplay(content: unknown): { type: string; text: string } {
  if (typeof content === "string") {
    return { type: "markdown", text: content };
  }
  if (content && typeof content === "object") {
    const obj = content as Record<string, unknown>;
    // SemanticGraphGrouping: has sourceNodeIds
    if (Array.isArray(obj.sourceNodeIds)) {
      return {
        type: "semantic_graph_grouping",
        text: JSON.stringify(
          {
            sourceNodeIds: obj.sourceNodeIds,
            targetGroupNodeId: obj.targetGroupNodeId,
            targetGroupLabel: obj.targetGroupLabel,
          },
          null,
          2,
        ),
      };
    }
    // MemoryPatch or other structured JSON
    return { type: "memory_patch", text: JSON.stringify(obj, null, 2) };
  }
  return { type: "unknown", text: JSON.stringify(content, null, 2) };
}
</script>

<template>
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-1">
      <h1
        class="truncate font-mono text-2xl font-semibold tracking-tight"
        :title="proposalId"
      >
        {{ shortId }}
      </h1>
      <p class="text-sm text-muted-foreground">
        Proposal details and review metadata.
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

    <Card v-else-if="store.error">
      <CardContent class="p-6">
        <Alert variant="destructive">{{ store.error }}</Alert>
      </CardContent>
    </Card>

    <template v-else-if="store.detail">
      <Card>
        <CardHeader>
          <CardTitle>{{ store.detail.title }}</CardTitle>
          <CardDescription>
            <Badge variant="secondary" class="text-[0.65rem]">
              {{ store.detail.targetType }}
            </Badge>
            <span class="ml-2 text-xs text-muted-foreground">
              {{ store.detail.createdAt }}
            </span>
          </CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <div>
            <h3 class="mb-1 text-sm font-medium">Rationale</h3>
            <p class="text-sm text-muted-foreground">
              {{ store.detail.rationale }}
            </p>
          </div>

          <div>
            <h3 class="mb-1 text-sm font-medium">Proposed Content</h3>
            <pre
              class="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-muted/20 p-3 font-mono text-xs"
            >{{ contentDisplay(store.detail.proposedContent).text }}</pre>
          </div>

          <div v-if="store.detail.evidence.length">
            <h3 class="mb-1 text-sm font-medium">Evidence</h3>
            <ul class="space-y-1.5">
              <li
                v-for="(link, i) in store.detail.evidence"
                :key="i"
                class="rounded-md border border-border bg-muted/10 px-3 py-2 text-xs"
              >
                <div class="font-medium">{{ link.sourceKind }}</div>
                <div class="text-muted-foreground">
                  subject: {{ link.subject }}
                </div>
                <div
                  v-if="link.rowIds.length"
                  class="text-muted-foreground"
                >
                  rowIds: {{ link.rowIds.join(", ") }}
                </div>
              </li>
            </ul>
          </div>
        </CardContent>
      </Card>

      <Card v-if="store.detail.reviewDecision">
        <CardHeader>
          <CardTitle>Review Decision</CardTitle>
        </CardHeader>
        <CardContent class="space-y-3">
          <div class="flex flex-wrap items-center gap-2">
            <Badge
              :variant="
                store.detail.reviewDecision.outcome === 'accepted'
                  ? 'default'
                  : 'destructive'
              "
            >
              {{ store.detail.reviewDecision.outcome }}
            </Badge>
            <span class="text-xs text-muted-foreground">
              by {{ store.detail.reviewDecision.reviewer }} on
              {{ store.detail.reviewDecision.decidedAt }}
            </span>
          </div>
          <div>
            <h3 class="mb-1 text-sm font-medium">Review Rationale</h3>
            <p class="text-sm text-muted-foreground">
              {{ store.detail.reviewDecision.rationale }}
            </p>
          </div>
        </CardContent>
      </Card>
    </template>
  </div>
</template>
