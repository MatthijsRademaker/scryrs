<script setup lang="ts">
defineProps<{ modelValue: string; options: { label: string; value: string }[] }>();
defineEmits<{ "update:modelValue": [value: string] }>();
</script>
<template>
  <select class="rounded-lg border border-input bg-card/50 px-3 py-2 text-sm text-foreground outline-none transition-shadow focus-visible:ring-2 focus-visible:ring-ring/50" :value="modelValue" @change="$emit('update:modelValue', ($event.target as HTMLSelectElement).value)">
    <option v-for="option in options" :key="option.value" :value="option.value" class="bg-card text-foreground">{{ option.label }}</option>
  </select>
</template>
