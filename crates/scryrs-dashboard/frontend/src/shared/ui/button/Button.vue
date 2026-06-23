<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/shared/lib/utils";

const props = withDefaults(defineProps<{ variant?: "default" | "outline" | "ghost"; class?: string }>(), {
  variant: "default",
  class: "",
});

const classes = computed(() =>
  cn(
    "inline-flex items-center justify-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-[background-color,box-shadow,color] duration-200 ease-out outline-none focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:shadow-[0_0_18px_-2px_var(--glow-accent)] disabled:pointer-events-none disabled:opacity-50",
    props.variant === "default" &&
      "bg-primary text-primary-foreground hover:bg-primary/90 hover:shadow-[0_0_20px_-4px_var(--glow-accent)]",
    props.variant === "outline" &&
      "border border-border bg-transparent text-foreground hover:border-primary/40 hover:bg-primary/10 hover:text-foreground",
    props.variant === "ghost" && "text-foreground hover:bg-foreground/5",
    props.class,
  ),
);
</script>

<template>
  <button :class="classes">
    <slot />
  </button>
</template>
