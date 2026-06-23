<script setup lang="ts">
import { computed } from "vue";
import { colorForKey } from "@/shared/lib/viz";

// Lightweight constellation: a center node ringed by satellites, joined by
// hairlines. Node size + glow scale by activity weight. Inline SVG only.
const props = withDefaults(
  defineProps<{
    nodes: { id: string; label: string; weight: number }[];
    centerLabel?: string;
    width?: number;
    height?: number;
    limit?: number;
  }>(),
  { centerLabel: "", width: 360, height: 240, limit: 8 },
);

const cx = computed(() => props.width / 2);
const cy = computed(() => props.height / 2);
const radius = computed(() => Math.min(props.width, props.height) / 2 - 56);

const ranked = computed(() =>
  [...(props.nodes ?? [])]
    .filter((n) => n && Number.isFinite(n.weight))
    .sort((a, b) => b.weight - a.weight),
);
const shown = computed(() => ranked.value.slice(0, props.limit));
const overflow = computed(() => Math.max(0, ranked.value.length - props.limit));
const maxWeight = computed(() => Math.max(1, ...shown.value.map((n) => n.weight)));

const satellites = computed(() =>
  shown.value.map((node, i) => {
    const angle = (i / Math.max(1, shown.value.length)) * Math.PI * 2 - Math.PI / 2;
    const x = cx.value + Math.cos(angle) * radius.value;
    const y = cy.value + Math.sin(angle) * radius.value;
    const ratio = node.weight / maxWeight.value;
    const r = 3.5 + ratio * 7;
    const color = colorForKey(node.id || node.label);
    // Label sits just outside the node, anchored away from center.
    const lx = cx.value + Math.cos(angle) * (radius.value + 14);
    const ly = cy.value + Math.sin(angle) * (radius.value + 14);
    const anchor = Math.abs(Math.cos(angle)) < 0.35 ? "middle" : Math.cos(angle) > 0 ? "start" : "end";
    return { ...node, x, y, r, color, ratio, lx, ly, anchor };
  }),
);
</script>

<template>
  <svg
    :width="'100%'"
    :viewBox="`0 0 ${width} ${height}`"
    class="max-w-full"
    role="img"
    :aria-label="`constellation of ${ranked.length} related nodes`"
  >
    <!-- hairlines from center to each satellite -->
    <line
      v-for="sat in satellites"
      :key="`edge-${sat.id}`"
      :x1="cx"
      :y1="cy"
      :x2="sat.x"
      :y2="sat.y"
      stroke="var(--border)"
      stroke-width="1"
      :stroke-opacity="0.4 + sat.ratio * 0.4"
    />

    <!-- center node -->
    <circle :cx="cx" :cy="cy" r="9" fill="var(--primary)" :style="{ filter: 'drop-shadow(0 0 10px var(--glow-accent))' }" />
    <circle :cx="cx" :cy="cy" r="15" fill="none" stroke="var(--primary)" stroke-opacity="0.35" stroke-width="1" />
    <text
      v-if="centerLabel"
      :x="cx"
      :y="cy + 30"
      text-anchor="middle"
      class="fill-foreground"
      style="font-size: 10px; font-weight: 600"
    >
      {{ centerLabel.length > 22 ? centerLabel.slice(0, 21) + "…" : centerLabel }}
    </text>

    <!-- satellites -->
    <g v-for="sat in satellites" :key="`node-${sat.id}`">
      <circle
        :cx="sat.x"
        :cy="sat.y"
        :r="sat.r"
        :fill="sat.color"
        :style="{ filter: `drop-shadow(0 0 ${(4 + sat.ratio * 8).toFixed(0)}px ${sat.color})`, opacity: 0.65 + sat.ratio * 0.35 }"
      />
      <text
        :x="sat.lx"
        :y="sat.ly + 3"
        :text-anchor="sat.anchor"
        class="fill-muted-foreground"
        style="font-size: 9px"
      >
        {{ sat.label.length > 16 ? sat.label.slice(0, 15) + "…" : sat.label }}
      </text>
    </g>

    <text
      v-if="overflow"
      :x="width - 6"
      :y="height - 6"
      text-anchor="end"
      class="fill-muted-foreground"
      style="font-size: 9px"
    >
      +{{ overflow }} more
    </text>
  </svg>
</template>
