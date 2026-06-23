<script setup lang="ts">
import { computed } from "vue";
import { colorForKey } from "@/shared/lib/viz";

// Event-type distribution as a single stacked bar, color-coded per type.
const props = withDefaults(
  defineProps<{ counts: Record<string, number>; legend?: boolean; legendLimit?: number }>(),
  { legend: false, legendLimit: 4 },
);

const segments = computed(() => {
  const entries = Object.entries(props.counts ?? {}).filter(([, n]) => n > 0);
  const total = entries.reduce((sum, [, n]) => sum + n, 0) || 1;
  return entries
    .sort((a, b) => b[1] - a[1])
    .map(([type, count]) => ({
      type,
      count,
      pct: (count / total) * 100,
      color: colorForKey(type),
    }));
});

const legendItems = computed(() => segments.value.slice(0, props.legendLimit));
const overflow = computed(() => Math.max(0, segments.value.length - props.legendLimit));
</script>

<template>
  <div class="flex flex-col gap-1.5">
    <div class="flex h-2 w-full overflow-hidden rounded-full bg-muted/50">
      <div
        v-for="seg in segments"
        :key="seg.type"
        class="h-full first:rounded-l-full last:rounded-r-full"
        :style="{ width: `${seg.pct}%`, backgroundColor: seg.color }"
        :title="`${seg.type}: ${seg.count}`"
      ></div>
      <span v-if="segments.length === 0" class="h-full w-full bg-muted/40"></span>
    </div>
    <div v-if="legend && legendItems.length" class="flex flex-wrap gap-x-3 gap-y-1 text-[0.7rem] text-muted-foreground">
      <span v-for="seg in legendItems" :key="seg.type" class="inline-flex items-center gap-1.5">
        <span class="size-1.5 rounded-full" :style="{ backgroundColor: seg.color }"></span>
        <span class="truncate">{{ seg.type }}</span>
        <span class="tabular-nums text-foreground/70">{{ seg.count }}</span>
      </span>
      <span v-if="overflow" class="text-muted-foreground/70">+{{ overflow }} more</span>
    </div>
  </div>
</template>
