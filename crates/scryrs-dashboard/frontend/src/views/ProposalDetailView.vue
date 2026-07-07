<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import {
  Alert,
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardDescription,
  EmptyState,
} from "@/shared/ui";
import { routeUnavailableMessage } from "@/shared/lib/dashboard-mode";
import { useProposalStore } from "@/stores/proposals";
import { useMetaStore } from "@/stores/meta";
import {
  canEditReviewedContent as canEditReviewedProposalContent,
  contentDisplay,
  reviewedContentPayload as buildReviewedContentPayload,
  reviewInputsValid as hasValidReviewInputs,
  showReviewForm as shouldShowReviewForm,
} from "@/views/proposal-detail";

const route = useRoute();
const store = useProposalStore();
const meta = useMetaStore();

const reviewer = ref("");
const rationale = ref("");
const decidedAt = ref("");
const reviewedContent = ref("");

const proposalId = computed(() => String(route.params.proposalId));
const shortId = computed(() =>
  proposalId.value.length > 18
    ? `${proposalId.value.slice(0, 18)}…`
    : proposalId.value,
);
const unavailableMessage = computed(() =>
  routeUnavailableMessage("proposal-detail", meta.mode),
);
const detailContent = computed(() =>
  store.detail ? contentDisplay(store.detail.proposedContent) : null,
);
const showReviewForm = computed(() =>
  shouldShowReviewForm(meta.isLiveMode, store.detail),
);
const canEditReviewedContent = computed(() =>
  canEditReviewedProposalContent(meta.isLiveMode, store.detail),
);
const reviewInputsValid = computed(() =>
  hasValidReviewInputs(reviewer.value, rationale.value, decidedAt.value),
);
const reviewedContentPayload = computed(() =>
  buildReviewedContentPayload(
    meta.isLiveMode,
    store.detail,
    reviewedContent.value,
  ),
);

onMounted(async () => {
  await meta.ensureLoaded();
  if (!meta.isLiveMode) {
    void store.loadProposal(proposalId.value);
  }
});

watch(
  () => store.detail,
  (detail) => {
    if (typeof detail?.proposedContent === "string") {
      reviewedContent.value = detail.proposedContent;
      return;
    }
    reviewedContent.value = "";
  },
  { immediate: true },
);

async function submitAccept() {
  await store.acceptProposal({
    proposalId: proposalId.value,
    reviewer: reviewer.value,
    rationale: rationale.value,
    decidedAt: decidedAt.value,
    reviewedContent: reviewedContentPayload.value,
  });
}

async function submitReject() {
  await store.rejectProposal({
    proposalId: proposalId.value,
    reviewer: reviewer.value,
    rationale: rationale.value,
    decidedAt: decidedAt.value,
  });
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
            >{{ detailContent?.text }}</pre>
          </div>

          <div v-if="showReviewForm" class="flex flex-col gap-4 rounded-md border border-border bg-muted/10 p-4">
            <div>
              <h3 class="mb-1 text-sm font-medium">Review Action</h3>
              <p class="text-sm text-muted-foreground">
                Enter explicit reviewer metadata before accepting or rejecting this proposal.
              </p>
            </div>

            <Alert v-if="store.reviewError" variant="destructive">
              {{ store.reviewError }}
            </Alert>
            <Alert v-else-if="store.reviewSuccess">
              {{ store.reviewSuccess }}
            </Alert>

            <label class="flex flex-col gap-1 text-sm">
              <span class="font-medium">Reviewer</span>
              <input
                v-model="reviewer"
                type="text"
                class="rounded-md border border-border bg-background px-3 py-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
              >
            </label>

            <label class="flex flex-col gap-1 text-sm">
              <span class="font-medium">Rationale</span>
              <textarea
                v-model="rationale"
                rows="3"
                class="rounded-md border border-border bg-background px-3 py-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
              />
            </label>

            <label class="flex flex-col gap-1 text-sm">
              <span class="font-medium">Decided At (RFC3339)</span>
              <input
                v-model="decidedAt"
                type="text"
                placeholder="2026-07-03T00:00:00Z"
                class="rounded-md border border-border bg-background px-3 py-2 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
              >
            </label>

            <label v-if="canEditReviewedContent" class="flex flex-col gap-1 text-sm">
              <span class="font-medium">Reviewed Markdown (optional)</span>
              <textarea
                v-model="reviewedContent"
                rows="10"
                class="rounded-md border border-border bg-background px-3 py-2 font-mono text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/60"
              />
            </label>

            <div class="flex flex-wrap gap-3">
              <Button :disabled="store.reviewLoading || !reviewInputsValid" @click="submitAccept">
                {{ store.reviewLoading ? "Submitting…" : "Accept proposal" }}
              </Button>
              <Button variant="outline" :disabled="store.reviewLoading || !reviewInputsValid" @click="submitReject">
                Reject proposal
              </Button>
            </div>
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
