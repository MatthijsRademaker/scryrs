import { describe, expect, it } from "vitest";
import {
	hotspotSubjectDisplay,
	navigationForMode,
	routeUnavailableMessage,
} from "@/shared/lib/dashboard-mode";

describe("navigationForMode", () => {
	it("shows signals and hides local-only routes in live mode", () => {
		expect(navigationForMode("live").map((item) => item.label)).toEqual([
			"Hotspots",
			"Signals",
			"About",
		]);
	});

	it("keeps existing local navigation in local mode", () => {
		expect(navigationForMode("local").map((item) => item.label)).toEqual([
			"Hotspots",
			"Sessions",
			"Events",
			"About",
		]);
	});
});

describe("hotspotSubjectDisplay", () => {
	const entry = {
		rank: 1,
		subjectKind: "file",
		subject: "/srv/repo/src/main.rs",
		score: 10,
		counts: { eventType: {}, outcome: {} },
		sessionCount: 1,
		firstSeen: "2026-06-29T19:00:00Z",
		lastSeen: "2026-06-29T19:00:00Z",
		evidence: { rowIds: [1] },
	};

	it("preserves the raw live subject", () => {
		expect(
			hotspotSubjectDisplay(entry, {
				mode: "live",
				repositoryPath: "/srv/repo",
			}).label,
		).toBe("/srv/repo/src/main.rs");
	});

	it("formats local file subjects relative to the repository root", () => {
		expect(
			hotspotSubjectDisplay(entry, {
				mode: "local",
				repositoryPath: "/srv/repo",
			}).label,
		).toBe("src/main.rs");
	});
});

describe("routeUnavailableMessage", () => {
	it("returns a live-mode explanation for local-only routes", () => {
		expect(routeUnavailableMessage("sessions", "live")).toContain(
			"not available in live mode",
		);
		expect(routeUnavailableMessage("events", "live")).toContain(
			"not available in live mode",
		);
	});

	it("keeps live-capable routes available", () => {
		expect(routeUnavailableMessage("hotspots", "live")).toBeNull();
		expect(routeUnavailableMessage("signals", "live")).toBeNull();
	});
});
