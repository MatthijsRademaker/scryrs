// Shared visualization helpers: stable, token-harmonized colors for event
// types and outcomes so every view/component color-codes data identically.

// A palette harmonized with the cyan identity (cool blues/teals/violets plus a
// couple of warm-leaning accents for contrast). Stays clear of the flame ramp,
// which is reserved for hotspot heat only.
export const EVENT_PALETTE = [
  "oklch(0.74 0.14 233)", // cyan
  "oklch(0.72 0.13 200)", // teal
  "oklch(0.70 0.15 278)", // violet
  "oklch(0.75 0.13 165)", // sea green
  "oklch(0.72 0.14 320)", // orchid
  "oklch(0.74 0.12 255)", // indigo
  "oklch(0.78 0.12 95)", // gold
  "oklch(0.72 0.13 145)", // green
] as const;

function hash(input: string): number {
  let h = 2166136261;
  for (let i = 0; i < input.length; i += 1) {
    h ^= input.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

/** Stable color for an arbitrary key (e.g. an event type). */
export function colorForKey(key: string): string {
  return EVENT_PALETTE[hash(key) % EVENT_PALETTE.length];
}

/** Semantic color for an outcome, falling back to neutral for unknowns. */
export function outcomeColor(outcome: string): string {
  const key = outcome.toLowerCase();
  if (/(^|_)(ok|pass|passed|success|succeeded|allow|allowed|complete|completed)($|_)/.test(key)) {
    return "var(--success)";
  }
  if (/(err|error|fail|failed|failure|deny|denied|reject|blocked|abort)/.test(key)) {
    return "var(--destructive)";
  }
  if (/(warn|warning|retry|skip|skipped|timeout|partial)/.test(key)) {
    return "var(--warning)";
  }
  return "var(--muted-foreground)";
}

/** Outcomes that warrant an attention pulse (errors/warnings). */
export function outcomeIsAlerting(outcome: string): boolean {
  const c = outcomeColor(outcome);
  return c === "var(--destructive)" || c === "var(--warning)";
}
