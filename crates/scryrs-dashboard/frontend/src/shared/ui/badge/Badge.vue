<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/shared/lib/utils";
const props = withDefaults(defineProps<{ variant?: "default" | "secondary" | "outline" | "destructive"; class?: string }>(), { variant: "default", class: "" });
const classes = computed(() => cn("inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium", props.variant === "default" && "border-transparent bg-primary text-primary-foreground", props.variant === "secondary" && "border-transparent bg-secondary text-secondary-foreground", props.variant === "outline" && "text-foreground", props.variant === "destructive" && "border-transparent bg-destructive text-destructive-foreground", props.class));
</script>
<template><span :class="classes"><slot /></span></template>
