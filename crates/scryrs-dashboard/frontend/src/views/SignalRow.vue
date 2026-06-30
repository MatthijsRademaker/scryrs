<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Motion, useReducedMotion } from "motion-v";
import { Badge, FlameIndicator } from "@/shared/ui";
import { nextIgnitionDelayMs } from "@/shared/lib/ignition";
import type { FeedSignal } from "@/stores/signals";

const props = defineProps<{ signal: FeedSignal; maxScore: number }>();

const reduced = useReducedMotion();
// A signal ignites only when it arrived live AND motion is allowed.
const ignites = computed(() => props.signal.live && reduced.value !== true);

// Resting heat: flame intensity scales with the signal's absolute score.
const intensity = computed(() =>
	Math.min(1, Math.max(0, props.signal.score / Math.max(1, props.maxScore))),
);

// Arrival drama scales with how hard the signal jumped past its threshold.
const heat = computed(() => {
	const threshold = props.signal.threshold || 0;
	const ratio =
		threshold > 0 ? props.signal.delta / threshold : props.signal.delta / 10;
	return Math.min(1, Math.max(0, ratio));
});

// Delta-driven flare magnitude + settle duration.
const flareOpacity = computed(() => 0.4 + heat.value * 0.45);
const flareDuration = computed(() => 0.5 + heat.value * 0.7);

// Cascade delay so a burst of live signals staggers instead of strobing.
const delaySeconds = ref(0);
const flareReady = ref(false);

// Score count-up: live rows tick from 0 → score; everything else shows it flat.
const display = ref(props.signal.score);
let rafId = 0;
let flareTimer: ReturnType<typeof setTimeout> | null = null;

const entranceInitial = computed(() =>
	ignites.value ? { opacity: 0, y: -14, scale: 0.97 } : { opacity: 0 },
);
const entranceAnimate = computed(() =>
	ignites.value ? { opacity: 1, y: 0, scale: 1 } : { opacity: 1 },
);
const entranceTransition = computed(() =>
	ignites.value
		? {
				type: "spring",
				stiffness: 480,
				damping: 30,
				mass: 0.9,
				delay: delaySeconds.value,
			}
		: { duration: 0.25, ease: "easeOut" },
);

function runCountUp(durationMs: number, delayMs: number) {
	const from = 0;
	const to = props.signal.score;
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
	if (!ignites.value) {
		display.value = props.signal.score;
		return;
	}
	const delayMs = nextIgnitionDelayMs(performance.now());
	delaySeconds.value = delayMs / 1000;
	const durationMs = 420 + heat.value * 500;
	runCountUp(durationMs, delayMs);
	flareTimer = setTimeout(() => {
		flareReady.value = true;
	}, delayMs);
});

onUnmounted(() => {
	if (rafId) cancelAnimationFrame(rafId);
	if (flareTimer !== null) clearTimeout(flareTimer);
});
</script>

<template>
  <Motion
    :layout="true"
    :initial="entranceInitial"
    :animate="entranceAnimate"
    :exit="{ opacity: 0, scale: 0.98 }"
    :transition="entranceTransition"
    class="glass-surface relative flex items-center gap-4 overflow-hidden rounded-xl px-4 py-3"
  >
    <!-- One-shot heat flare on live arrival; magnitude scales with delta. -->
    <Motion
      v-if="ignites && flareReady"
      class="pointer-events-none absolute inset-0 rounded-xl"
      :initial="{ opacity: flareOpacity }"
      :animate="{ opacity: 0 }"
      :transition="{ duration: flareDuration, ease: 'easeOut' }"
      :style="{
        boxShadow: '0 0 30px 2px var(--glow-hot)',
        backgroundImage:
          'radial-gradient(120% 140% at 0% 50%, var(--glow-hot), transparent 62%)',
      }"
    />

    <Badge variant="outline" class="shrink-0 font-mono tabular-nums">#{{ signal.id }}</Badge>

    <div class="min-w-0 flex-1">
      <div class="truncate font-medium text-foreground" :title="signal.subject">
        {{ signal.subject }}
      </div>
      <div class="mt-0.5 truncate text-xs text-muted-foreground">
        {{ signal.subjectKind }} · threshold {{ signal.threshold }} · Δ {{ signal.delta }} ·
        {{ signal.createdAt }}
      </div>
    </div>

    <div class="flex shrink-0 flex-col items-end gap-1.5">
      <span class="text-flame text-2xl font-semibold tabular-nums">{{ display }}</span>
      <div class="w-28"><FlameIndicator :intensity="intensity" :show-icon="false" /></div>
    </div>
  </Motion>
</template>
