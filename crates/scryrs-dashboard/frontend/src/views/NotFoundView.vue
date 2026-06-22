<script setup lang="ts">
import { RouterLink } from "vue-router";
import { Button, Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/shared/ui";
</script>
<template>
  <Card>
    <CardHeader><CardTitle>Page not found</CardTitle><CardDescription>The dashboard route does not exist.</CardDescription></CardHeader>
    <CardContent><RouterLink to="/" custom v-slot="{ navigate }"><Button @click="navigate">Back to hotspots</Button></RouterLink></CardContent>
  </Card>
</template>
