<script setup lang="ts">
defineProps<{ title: string; description: string }>();
</script>
<template>
  <div class="flex min-h-48 flex-col items-center justify-center gap-2 rounded-lg border border-dashed bg-card p-8 text-center">
    <h3 class="font-semibold">{{ title }}</h3>
    <p class="max-w-md text-sm text-muted-foreground">{{ description }}</p>
    <slot />
  </div>
</template>
