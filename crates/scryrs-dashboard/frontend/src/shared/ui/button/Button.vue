<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/shared/lib/utils";

const props = withDefaults(defineProps<{ variant?: "default" | "outline" | "ghost"; class?: string }>(), {
  variant: "default",
  class: "",
});

const classes = computed(() => cn(
  "inline-flex items-center justify-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50",
  props.variant === "default" && "bg-primary text-primary-foreground hover:bg-primary/90",
  props.variant === "outline" && "border bg-background hover:bg-accent hover:text-accent-foreground",
  props.variant === "ghost" && "hover:bg-accent hover:text-accent-foreground",
  props.class,
));
</script>

<template>
  <button :class="classes">
    <slot />
  </button>
</template>
