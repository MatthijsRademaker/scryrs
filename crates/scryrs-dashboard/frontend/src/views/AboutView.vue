<script setup lang="ts">
import { computed, onMounted } from "vue";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/shared/ui";
import { useMetaStore } from "@/stores/meta";

const meta = useMetaStore();
const modeLabel = computed(() => meta.isLiveMode ? "Live dashboard proxy" : "Local artifact viewer");
const sourceCopy = computed(() => {
  if (meta.isLiveMode) {
    return `Data source: live hotspot rankings and signals proxied through this dashboard${meta.repositoryId ? ` for ${meta.repositoryId}` : ""}.`;
  }
  return "Data source: .scryrs/hotspots.json and .scryrs/scryrs.db in the current repository.";
});
const footerCopy = computed(() => meta.isLiveMode
  ? "No browser-direct server access, mutation API, authentication, or local/live data merging is included in this mode."
  : "No hosted service, mutation API, authentication, or real-time streaming is included in local mode.");

onMounted(() => {
  void meta.ensureLoaded();
});
</script>
<template>
  <div class="flex flex-col gap-6">
    <header class="flex items-center gap-3">
      <span aria-hidden="true" class="text-primary drop-shadow-[0_0_10px_var(--glow-accent)]">◆</span>
      <div class="flex flex-col gap-1">
        <h1 class="text-2xl font-semibold tracking-[0.2em]">scryrs</h1>
        <p class="text-sm text-muted-foreground">{{ modeLabel }}</p>
      </div>
    </header>
    <Card>
      <CardHeader><CardTitle>About</CardTitle><CardDescription>Version 0.1.0</CardDescription></CardHeader>
      <CardContent class="flex flex-col gap-3 text-sm text-foreground/90">
        <p>{{ sourceCopy }}</p>
        <p>Docs: <a href="/project-docs/roadmap" class="text-primary hover:underline">Project roadmap</a></p>
        <p class="text-muted-foreground">{{ footerCopy }}</p>
      </CardContent>
    </Card>
  </div>
</template>
