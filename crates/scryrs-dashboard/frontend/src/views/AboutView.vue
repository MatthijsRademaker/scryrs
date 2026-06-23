<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/shared/ui";
</script>
<template>
  <div class="flex flex-col gap-6">
    <header class="flex items-center gap-3">
      <span aria-hidden="true" class="text-primary drop-shadow-[0_0_10px_var(--glow-accent)]">◆</span>
      <div class="flex flex-col gap-1">
        <h1 class="text-2xl font-semibold tracking-[0.2em]">scryrs</h1>
        <p class="text-sm text-muted-foreground">Local-only visual browser for .scryrs trace artifacts.</p>
      </div>
    </header>
    <Card>
      <CardHeader><CardTitle>About</CardTitle><CardDescription>Version 0.1.0</CardDescription></CardHeader>
      <CardContent class="flex flex-col gap-3 text-sm text-foreground/90">
        <p>Data source: <code class="rounded bg-card/60 px-1.5 py-0.5 text-foreground">.scryrs/hotspots.json</code> and <code class="rounded bg-card/60 px-1.5 py-0.5 text-foreground">.scryrs/scryrs.db</code> in the current repository.</p>
        <p>Docs: <a href="/project-docs/roadmap" class="text-primary hover:underline">Project roadmap</a></p>
        <p class="text-muted-foreground">No hosted service, mutation API, authentication, or real-time streaming is included in Phase 3.</p>
      </CardContent>
    </Card>
  </div>
</template>
