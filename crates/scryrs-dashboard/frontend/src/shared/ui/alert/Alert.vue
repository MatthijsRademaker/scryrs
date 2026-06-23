<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/shared/lib/utils";
const props = withDefaults(defineProps<{ variant?: "default" | "destructive"; class?: string }>(), { variant: "default", class: "" });
const classes = computed(() => cn("rounded-xl border p-4 text-sm", props.variant === "default" && "border-border bg-card/40", props.variant === "destructive" && "border-destructive/40 bg-destructive/10 text-destructive", props.class));
</script>
<template><div :class="classes"><slot /></div></template>
