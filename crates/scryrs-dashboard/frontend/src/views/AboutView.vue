<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/shared/ui";
</script>
<template>
  <Card>
    <CardHeader><CardTitle>About scryrs Dashboard</CardTitle><CardDescription>Version 0.1.0 local-only visual browser.</CardDescription></CardHeader>
    <CardContent class="flex flex-col gap-3 text-sm">
      <p>Data source: <code>.scryrs/hotspots.json</code> and <code>.scryrs/scryrs.db</code> in current repository.</p>
      <p>Docs: <a href="/project-docs/roadmap" class="text-primary">Project roadmap</a></p>
      <p>No hosted service, mutation API, authentication, or real-time streaming is included in Phase 3.</p>
    </CardContent>
  </Card>
</template>
