<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/shared/lib/utils";

type Variant = "default" | "secondary" | "outline" | "destructive" | "success" | "warning" | "info";
const props = withDefaults(defineProps<{ variant?: Variant; class?: string }>(), {
  variant: "default",
  class: "",
});

const classes = computed(() =>
  cn(
    "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium tracking-tight transition-colors",
    props.variant === "default" && "border-primary/30 bg-primary/15 text-primary",
    props.variant === "secondary" && "border-transparent bg-secondary text-secondary-foreground",
    props.variant === "outline" && "border-border text-foreground",
    props.variant === "destructive" && "border-destructive/30 bg-destructive/15 text-destructive",
    props.variant === "success" && "border-success/30 bg-success/15 text-success",
    props.variant === "warning" && "border-warning/30 bg-warning/15 text-warning",
    props.variant === "info" && "border-info/30 bg-info/15 text-info",
    props.class,
  ),
);
</script>

<template>
  <span :class="classes"><slot /></span>
</template>
