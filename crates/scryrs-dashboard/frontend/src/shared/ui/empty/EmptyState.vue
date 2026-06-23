<script setup lang="ts">
defineProps<{ title: string; description: string }>();
</script>
<template>
  <div class="flex min-h-48 flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-border bg-card/20 p-8 text-center">
    <h3 class="font-semibold text-foreground">{{ title }}</h3>
    <p class="max-w-md text-sm text-muted-foreground">{{ description }}</p>
    <slot />
  </div>
</template>
