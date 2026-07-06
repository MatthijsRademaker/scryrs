<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { Alert, Badge, Card, CardContent, CardHeader, CardTitle, CardDescription, EmptyState } from "@/shared/ui";
import { routeUnavailableMessage } from "@/shared/lib/dashboard-mode";
import { useAcceptedStore } from "@/stores/accepted";
import { useMetaStore } from "@/stores/meta";
import type { PublishSurfaceStatus } from "@/shared/api/client";

const route = useRoute();
const store = useAcceptedStore();
const meta = useMetaStore();

const proposalId = computed(() => String(route.params.proposalId));
const shortId = computed(() =>
  proposalId.value.length > 18
    ? `${proposalId.value.slice(0, 18)}…`
    : proposalId.value,
);
const unavailableMessage = computed(() =>
  routeUnavailableMessage("accepted-detail", meta.mode),
);

onMounted(async () => {
  await meta.ensureLoaded();
  if (!meta.isLiveMode) {
    void store.fetchDetail(proposalId.value);
  }
});

function contentDisplay(content: unknown): { type: string; text: string } {
  if (typeof content === "string") {
    return { type: "markdown", text: content };
  }
  if (content && typeof content === "object") {
    const obj = content as Record<string, unknown>;
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
    return { type: "memory_patch", text: JSON.stringify(obj, null, 2) };
  }
  return { type: "unknown", text: JSON.stringify(content, null, 2) };
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
      <h1
        class="truncate font-mono text-2xl font-semibold tracking-tight"
        :title="proposalId"
      >
        {{ shortId }}
      </h1>
      <p class="text-sm text-muted-foreground">
        Accepted review decision with publish status.
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

    <Card v-else-if="store.detailError">
      <CardContent class="p-6">
        <Alert variant="destructive">{{ store.detailError }}</Alert>
      </CardContent>
    </Card>

    <template v-else-if="store.selectedItem">
      <Card>
        <CardHeader>
          <CardTitle>{{ store.selectedItem.title }}</CardTitle>
          <CardDescription>
            <Badge variant="secondary" class="text-[0.65rem]">
              {{ store.selectedItem.targetType }}
            </Badge>
            <span class="ml-2 text-xs text-muted-foreground">
              accepted {{ store.selectedItem.decidedAt }}
            </span>
          </CardDescription>
        </CardHeader>
        <CardContent class="space-y-4">
          <div>
            <h3 class="mb-1 text-sm font-medium">Accepted Content</h3>
            <pre
              class="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-muted/20 p-3 font-mono text-xs"
            >{{ contentDisplay(store.selectedItem.acceptedContent).text }}</pre>
          </div>

          <div
            v-if="
              store.selectedItem.originalProposedContent !== undefined &&
              store.selectedItem.originalProposedContent !== null
            "
          >
            <h3 class="mb-1 text-sm font-medium text-amber-400">
              ⚠ Original Proposed Content (differs from accepted)
            </h3>
            <pre
              class="max-h-60 overflow-auto whitespace-pre-wrap break-words rounded-md border border-amber-500/30 bg-amber-500/5 p-3 font-mono text-xs"
            >{{ contentDisplay(store.selectedItem.originalProposedContent).text }}</pre>
          </div>

          <div>
            <h3 class="mb-1 text-sm font-medium">Reviewer</h3>
            <p class="text-sm text-muted-foreground">
              {{ store.selectedItem.reviewer }}
            </p>
          </div>

          <div>
            <h3 class="mb-1 text-sm font-medium">Rationale</h3>
            <p class="text-sm text-muted-foreground">
              {{ store.selectedItem.rationale }}
            </p>
          </div>

          <div>
            <h3 class="mb-1 text-sm font-medium">
              Publish Status
            </h3>
            <div class="flex flex-wrap gap-2">
              <div
                v-for="(ps, idx) in store.selectedItem.publishStatus"
                :key="idx"
                class="flex items-center gap-1.5"
              >
                <Badge :variant="publishBadgeVariant(ps.status)" class="text-[0.6rem]">
                  {{ publishBadgeLabel(ps.surface, ps.status) }}
                </Badge>
                <span
                  v-if="ps.reason"
                  class="text-xs text-muted-foreground"
                >
                  {{ ps.reason }}
                </span>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card v-if="store.selectedItem.evidence.length">
        <CardHeader>
          <CardTitle>Evidence</CardTitle>
        </CardHeader>
        <CardContent>
          <ul class="space-y-1.5">
            <li
              v-for="(link, i) in store.selectedItem.evidence"
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
        </CardContent>
      </Card>
    </template>
  </div>
</template>
