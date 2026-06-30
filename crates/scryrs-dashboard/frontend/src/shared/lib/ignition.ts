// Serializes signal "ignition" animations so a rapid live burst cascades
// gracefully instead of flaring all at once (a strobe). Shared module state:
// each ignition reserves a slot at least IGNITION_STAGGER_MS after the last.

const IGNITION_STAGGER_MS = 120;
// Cap the cascade so a very large burst doesn't push later ignitions absurdly
// far into the future — beyond this lead, just ignite immediately.
const MAX_IGNITION_LEAD_MS = 600;

let lastIgnitionAt = 0;

/**
 * Returns the delay (ms) this ignition should wait before animating, given the
 * current time, and reserves its slot. Returns 0 when nothing recent is queued.
 */
export function nextIgnitionDelayMs(now: number): number {
	const earliest = lastIgnitionAt + IGNITION_STAGGER_MS;
	let delay = Math.max(0, earliest - now);
	if (delay > MAX_IGNITION_LEAD_MS) {
		// Burst is too dense to stagger politely; collapse rather than lag.
		delay = 0;
		lastIgnitionAt = now;
		return 0;
	}
	lastIgnitionAt = now + delay;
	return delay;
}

/** Test seam: reset the shared cascade clock. */
export function resetIgnitionClock(): void {
	lastIgnitionAt = 0;
}
