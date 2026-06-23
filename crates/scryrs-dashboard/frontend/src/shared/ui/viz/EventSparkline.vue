<script setup lang="ts">
import { computed } from "vue";

// Inline-SVG event activity. With a real series (>=2 points) it draws an
// area+line sparkline; with a single datum it degrades to a magnitude bar
// (used by the Sessions list, which exposes only an event count per row).
const props = withDefaults(
  defineProps<{ values: number[]; max?: number; width?: number; height?: number }>(),
  { width: 104, height: 28 },
);

const pad = 3;
const series = computed(() => (props.values ?? []).map((v) => (Number.isFinite(v) ? Math.max(0, v) : 0)));
const peak = computed(() => Math.max(props.max ?? 0, ...series.value, 1));
const isSeries = computed(() => series.value.length >= 2);

const points = computed(() => {
  const n = series.value.length;
  const innerW = props.width - pad * 2;
  const innerH = props.height - pad * 2;
  return series.value.map((v, i) => {
    const x = pad + (n <= 1 ? innerW / 2 : (i / (n - 1)) * innerW);
    const y = pad + innerH - (v / peak.value) * innerH;
    return [x, y] as const;
  });
});

const linePath = computed(() => points.value.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" "));
const areaPath = computed(() => {
  const pts = points.value;
  if (pts.length < 2) return "";
  const base = props.height - pad;
  const first = pts[0];
  const last = pts[pts.length - 1];
  return `M ${first[0].toFixed(1)},${base} L ${pts.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(" L ")} L ${last[0].toFixed(1)},${base} Z`;
});
const end = computed(() => points.value[points.value.length - 1]);

// Single-datum magnitude bar
const barWidth = computed(() => {
  const innerW = props.width - pad * 2;
  return (Math.min(1, (series.value[0] ?? 0) / peak.value) * innerW).toFixed(1);
});
</script>

<template>
  <svg
    :width="width"
    :height="height"
    :viewBox="`0 0 ${width} ${height}`"
    class="overflow-visible"
    role="img"
    aria-label="event activity"
  >
    <template v-if="isSeries">
      <path :d="areaPath" fill="var(--primary)" fill-opacity="0.14" />
      <polyline
        :points="linePath"
        fill="none"
        stroke="var(--primary)"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
      <circle
        v-if="end"
        :cx="end[0]"
        :cy="end[1]"
        r="2.2"
        fill="var(--primary)"
        :style="{ filter: 'drop-shadow(0 0 5px var(--glow-accent))' }"
      />
    </template>
    <template v-else>
      <rect :x="pad" :y="height / 2 - 3" :width="width - pad * 2" height="6" rx="3" fill="var(--muted)" />
      <rect
        :x="pad"
        :y="height / 2 - 3"
        :width="barWidth"
        height="6"
        rx="3"
        fill="var(--primary)"
        :style="{ filter: 'drop-shadow(0 0 5px var(--glow-accent))' }"
      />
    </template>
  </svg>
</template>
