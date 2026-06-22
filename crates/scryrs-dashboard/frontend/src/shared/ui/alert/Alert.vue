<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/shared/lib/utils";
const props = withDefaults(defineProps<{ variant?: "default" | "destructive"; class?: string }>(), { variant: "default", class: "" });
const classes = computed(() => cn("rounded-lg border p-4 text-sm", props.variant === "default" && "bg-card", props.variant === "destructive" && "border-destructive bg-destructive/10 text-destructive", props.class));
</script>
<template><div :class="classes"><slot /></div></template>
