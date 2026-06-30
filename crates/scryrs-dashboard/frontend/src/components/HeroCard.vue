<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { RouterLink } from "vue-router";
import { Motion, useReducedMotion } from "motion-v";
import { Badge, EventTypeBar, OutcomePulse } from "@/shared/ui";
import { nextIgnitionDelayMs } from "@/shared/lib/ignition";
import type { DeltaEntry } from "@/stores/hotspots";

const props = defineProps<{
  entry: DeltaEntry;
  maxScore: number;
  subjectLabel: string;
  isExternal: boolean;
}>();

const reduced = useReducedMotion();
const ignites = computed(() => props.entry.deltaType !== "unchanged" && reduced.value !== true);

// Entrance spring: only for new entrants on subsequent polls.
const entranceInitial = computed(() =>
  props.entry.deltaType === "entered" && ignites.value
    ? { opacity: 0, y: -14, scale: 0.97 }
    : { opacity: 1, y: 0, scale: 1 },
);
const entranceAnimate = computed(() => ({ opacity: 1, y: 0, scale: 1 }));

// Stagger entrance via shared ignition clock.
const delaySeconds = ref(0);

const entranceTransition = computed(() =>
  props.entry.deltaType === "entered" && ignites.value
    ? {
        type: "spring" as const,
        stiffness: 480,
        damping: 30,
        mass: 0.9,
        delay: delaySeconds.value,
      }
    : { duration: 0.25, ease: "easeOut" as const },
);

// Score count-up for score increases.
const display = ref(props.entry.score);
let rafId = 0;

function runCountUp(from: number, to: number, durationMs: number, delayMs: number) {
  display.value = from;
  const begin = performance.now() + delayMs;
  const step = (now: number) => {
    const elapsed = now - begin;
    if (elapsed < 0) {
      rafId = requestAnimationFrame(step);
      return;
    }
    const p = Math.min(1, elapsed / durationMs);
    const eased = 1 - Math.pow(1 - p, 3); // ease-out cubic
    display.value = Math.round(from + (to - from) * eased);
    if (p < 1) rafId = requestAnimationFrame(step);
  };
  rafId = requestAnimationFrame(step);
}

onMounted(() => {
  if (props.entry.deltaType === "entered" && ignites.value) {
    delaySeconds.value = nextIgnitionDelayMs(performance.now()) / 1000;
  }
  if (props.entry.scoreIncreased && ignites.value) {
    // Count up from zero to new score, capped at 800ms.
    const durationMs = Math.min(800, 300 + Math.abs(props.entry.score) * 10);
    const delayMs = props.entry.deltaType === "entered" ? nextIgnitionDelayMs(performance.now()) : 0;
    runCountUp(0, props.entry.score, durationMs, delayMs);
  } else {
    display.value = props.entry.score;
  }
});

onUnmounted(() => {
  if (rafId) cancelAnimationFrame(rafId);
});
</script>

<template>
  <Motion
    :layout="true"
    :initial="entranceInitial"
    :animate="entranceAnimate"
    :exit="{ opacity: 0, scale: 0.98 }"
    :transition="entranceTransition"
    class="flex flex-col gap-3 rounded-xl border border-border bg-card p-5 transition-shadow duration-300 hover:shadow-[0_0_34px_-14px_var(--glow-hot)]"
  >
    <div class="flex items-start justify-between gap-2">
      <Badge variant="outline" class="font-mono">#{{ entry.rank }}</Badge>
      <span class="truncate text-xs uppercase tracking-wider text-muted-foreground">{{ entry.subjectKind }}</span>
    </div>
    <RouterLink
      :to="{ name: 'subject-detail', params: { subjectKind: entry.subjectKind, subject: entry.subject } }"
      class="flex items-center gap-1.5 truncate text-lg font-semibold text-foreground no-underline transition-colors hover:text-primary"
      :title="entry.subject"
    >
      <Badge v-if="isExternal" variant="warning" class="shrink-0">EXTERNAL</Badge>
      <span class="truncate">{{ subjectLabel }}</span>
    </RouterLink>
    <div class="flex items-end justify-between gap-3">
      <span class="text-flame text-3xl font-semibold tabular-nums">{{ display }}</span>
      <div class="flex-1 pb-2">
        <div class="flex h-1.5 w-full rounded-full bg-muted">
          <div
            class="h-full rounded-full bg-flame transition-all duration-500 ease-out"
            :style="{ width: `${(entry.score / Math.max(1, maxScore)) * 100}%` }"
          />
        </div>
      </div>
    </div>
    <EventTypeBar :counts="entry.counts.eventType" legend />
    <OutcomePulse :counts="entry.counts.outcome" />
    <div class="mt-1 flex items-center justify-between border-t border-border pt-3 text-xs text-muted-foreground">
      <span><span class="tabular-nums text-foreground/80">{{ entry.sessionCount }}</span> sessions</span>
      <span><span class="tabular-nums text-foreground/80">{{ entry.score }}</span> events</span>
    </div>
  </Motion>
</template>
