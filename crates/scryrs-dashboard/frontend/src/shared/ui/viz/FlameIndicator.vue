<script setup lang="ts">
import { computed } from "vue";
import IconFlame from "@/shared/ui/icon/IconFlame.vue";

// Heat indicator — flame gradient + glow scaled by `intensity` (0..1).
// Reserved for hotspot heat per the "data glows" discipline.
const props = withDefaults(
  defineProps<{ intensity: number; showIcon?: boolean }>(),
  { showIcon: true },
);

const clamped = computed(() => Math.min(1, Math.max(0, props.intensity || 0)));
const widthPct = computed(() => `${Math.round(8 + clamped.value * 92)}%`);
// Glow strengthens with heat; stays subtle at the low end.
const barGlow = computed(
  () => `0 0 ${Math.round(4 + clamped.value * 16)}px ${0.2 + clamped.value * 0.5}px var(--glow-hot)`,
);
const iconGlow = computed(
  () => `drop-shadow(0 0 ${Math.round(2 + clamped.value * 10)}px var(--glow-hot))`,
);
const iconOpacity = computed(() => 0.5 + clamped.value * 0.5);
</script>

<template>
  <div class="flex items-center gap-2" :title="`Heat ${Math.round(clamped * 100)}%`">
    <IconFlame
      v-if="showIcon"
      class="size-4 shrink-0"
      :style="{ color: 'var(--flame-via)', filter: iconGlow, opacity: iconOpacity }"
    />
    <div class="h-2 w-full overflow-hidden rounded-full bg-muted/60">
      <div
        class="bg-flame h-full rounded-full transition-[width] duration-300 ease-out"
        :style="{ width: widthPct, boxShadow: barGlow }"
      ></div>
    </div>
  </div>
</template>
