<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/shared/lib/utils";

const props = withDefaults(defineProps<{ class?: string }>(), { class: "" });
const classes = computed(() =>
  cn("glass-surface rounded-xl text-card-foreground", props.class),
);
</script>

<template>
  <section :class="classes"><slot /></section>
</template>
