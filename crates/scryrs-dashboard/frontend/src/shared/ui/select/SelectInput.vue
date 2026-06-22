<script setup lang="ts">
defineProps<{ modelValue: string; options: { label: string; value: string }[] }>();
defineEmits<{ "update:modelValue": [value: string] }>();
</script>
<template>
  <select class="rounded-md border bg-background px-3 py-2 text-sm" :value="modelValue" @change="$emit('update:modelValue', ($event.target as HTMLSelectElement).value)">
    <option v-for="option in options" :key="option.value" :value="option.value">{{ option.label }}</option>
  </select>
</template>
