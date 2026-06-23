<script setup lang="ts">
import { computed } from "vue";
import { outcomeColor, outcomeIsAlerting } from "@/shared/lib/viz";

// Outcome distribution as small "pulse" chips; alerting outcomes pulse.
const props = defineProps<{ counts: Record<string, number> }>();

const chips = computed(() =>
  Object.entries(props.counts ?? {})
    .filter(([, n]) => n > 0)
    .sort((a, b) => b[1] - a[1])
    .map(([outcome, count]) => ({
      outcome,
      count,
      color: outcomeColor(outcome),
      alerting: outcomeIsAlerting(outcome),
    })),
);
</script>

<template>
  <div v-if="chips.length" class="flex flex-wrap items-center gap-1.5">
    <span
      v-for="chip in chips"
      :key="chip.outcome"
      class="inline-flex items-center gap-1.5 rounded-full border border-border bg-card/40 px-2 py-0.5 text-[0.7rem] font-medium"
      :title="`${chip.outcome}: ${chip.count}`"
    >
      <span
        class="size-1.5 rounded-full"
        :class="chip.alerting ? 'pulse-dot' : ''"
        :style="{ backgroundColor: chip.color, color: chip.color }"
      ></span>
      <span class="text-foreground/90">{{ chip.outcome }}</span>
      <span class="tabular-nums text-muted-foreground">{{ chip.count }}</span>
    </span>
  </div>
</template>
