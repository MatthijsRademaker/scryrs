<script setup lang="ts">
import { RouterLink } from "vue-router";
import { Button, Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/shared/ui";
</script>
<template>
  <Card>
    <CardHeader><CardTitle>Page not found</CardTitle><CardDescription>The dashboard route does not exist.</CardDescription></CardHeader>
    <CardContent class="p-6 pt-0"><RouterLink to="/" custom v-slot="{ navigate }"><Button @click="navigate">Back to hotspots</Button></RouterLink></CardContent>
  </Card>
</template>
