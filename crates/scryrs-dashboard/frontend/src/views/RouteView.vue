<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Alert, Button, Card, CardContent, EmptyState } from "@/shared/ui";
import { routeUnavailableMessage } from "@/shared/lib/dashboard-mode";
import { useRouteStore } from "@/stores/routes";
import { useMetaStore } from "@/stores/meta";
import type { EvidenceLink } from "@/shared/api/client";

const store = useRouteStore();
const meta = useMetaStore();
const searchInput = ref("");
const unavailableMessage = computed(() =>
  routeUnavailableMessage("routes", meta.mode),
);

onMounted(async () => {
  await meta.ensureLoaded();
});

function onSearch() {
  void store.search(searchInput.value);
}

function hasResults() {
  return !store.loading && store.error === null && store.hints.length > 0;
}

function isZeroMatch() {
  return (
    !store.loading &&
    store.error === null &&
    store.hints.length === 0 &&
    store.query.trim().length > 0
  );
}

function is404Error(): boolean {
  return store.error !== null && store.error.includes("404");
}

function is502Error(): boolean {
  return store.error !== null && store.error.includes("502");
}

function evidenceText(link: EvidenceLink): string {
  const parts = [link.sourceKind, link.subject];
  if (link.rowIds.length > 0) {
    parts.push(`[${link.rowIds.join(", ")}]`);
  }
  return parts.join(" / ");
}

function relevanceDisplay(relevance: number | undefined): string {
  if (relevance === undefined) return "—";
  return relevance.toString();
}
</script>

<template>
  <div class="flex flex-col gap-6">
    <header class="flex flex-col gap-1">
      <h1 class="text-2xl font-semibold tracking-tight">Routes</h1>
      <p class="text-sm text-muted-foreground">
        Search what future agents would load and why — driven by deterministic
        route explain hints.
      </p>
    </header>

    <!-- Unavailable in live mode -->
    <Card v-if="unavailableMessage">
      <CardContent class="p-6">
        <EmptyState
          title="Unavailable in live mode"
          :description="unavailableMessage"
        />
      </CardContent>
    </Card>

    <!-- Local mode -->
    <template v-else>
      <Card>
        <CardContent class="flex flex-col gap-4 p-6">
          <!-- Search bar -->
          <form
            class="flex items-center gap-3"
            @submit.prevent="onSearch"
          >
            <input
              v-model="searchInput"
              type="text"
              placeholder="Enter a query to search route hints…"
              class="flex-1 rounded-lg border border-border bg-muted/30 px-4 py-2 text-sm placeholder:text-muted-foreground focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
              :disabled="store.loading"
            />
            <Button
              type="submit"
              :disabled="searchInput.trim().length === 0 || store.loading"
            >
              Search
            </Button>
          </form>
        </CardContent>
      </Card>

      <!-- Loading state -->
      <Card v-if="store.loading">
        <CardContent class="flex items-center justify-center gap-2 p-6 text-sm text-muted-foreground">
          <span class="size-3 animate-spin rounded-full border-2 border-primary border-t-transparent"></span>
          Searching route hints…
        </CardContent>
      </Card>

      <!-- Error states -->
      <Alert v-else-if="store.error" variant="destructive">
        <template v-if="is404Error()">
          <p>
            Route artifact not found. Run
            <code class="rounded bg-muted px-1 py-0.5 text-xs">scryrs route &lt;PATH&gt;</code>
            to generate the route manifest.
          </p>
        </template>
        <template v-else-if="is502Error()">
          <p>
            Route artifact is malformed or from an incompatible version. Run
            <code class="rounded bg-muted px-1 py-0.5 text-xs">scryrs route &lt;PATH&gt;</code>
            to regenerate.
          </p>
        </template>
        <template v-else>
          {{ store.error }}
        </template>
      </Alert>

      <!-- Zero-match state -->
      <Card v-else-if="isZeroMatch()">
        <CardContent class="p-6">
          <EmptyState
            title="No routes matched"
            description="No route entries matched your query. Try a different search term."
          />
        </CardContent>
      </Card>

      <!-- Initial prompt state -->
      <Card v-else-if="store.query.trim().length === 0">
        <CardContent class="p-6">
          <EmptyState
            title="Search route hints"
            description="Enter a query above to find matching route entries and see what future agents would load."
          />
        </CardContent>
      </Card>

      <!-- Results -->
      <div v-else-if="hasResults()" class="flex flex-col gap-2.5">
        <div class="text-sm text-muted-foreground">
          {{ store.hints.length }} hint{{ store.hints.length === 1 ? "" : "s" }}
          for "{{ store.query }}"
        </div>
        <Card
          v-for="(hint, index) in store.hints"
          :key="hint.routeId"
          class="scry-in flex flex-col gap-2 p-4 transition-shadow duration-300 hover:shadow-[0_0_28px_-16px_var(--glow-accent)]"
          :style="{ animationDelay: `${Math.min(index, 12) * 40}ms` }"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span
              class="inline-flex size-5 items-center justify-center rounded bg-muted text-xs font-mono font-medium text-foreground"
            >
              {{ hint.rank }}
            </span>
            <span class="text-sm font-semibold">{{ hint.label }}</span>
            <span class="font-mono text-xs text-muted-foreground">{{
              hint.target
            }}</span>
            <span
              v-if="hint.loadTarget"
              class="inline-flex items-center gap-1 rounded border border-border px-1.5 py-0.5 text-[0.65rem] text-muted-foreground"
            >
              {{ hint.loadTarget.kind }}
              <template v-if="hint.loadTarget.reference">
                · {{ hint.loadTarget.reference }}
              </template>
            </span>
          </div>
          <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
            <span>
              relevance: {{ relevanceDisplay(hint.relevance) }}
            </span>
            <span class="max-w-prose truncate" :title="hint.reason">
              {{ hint.reason }}
            </span>
          </div>
          <div v-if="hint.evidence && hint.evidence.length > 0" class="flex flex-col gap-1">
            <div
              v-for="(link, linkIdx) in hint.evidence"
              :key="linkIdx"
              class="text-[0.7rem] text-muted-foreground/70"
            >
              {{ evidenceText(link) }}
            </div>
          </div>
        </Card>
      </div>
    </template>
  </div>
</template>
