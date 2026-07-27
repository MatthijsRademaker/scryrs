import { describe, expect, it } from "vitest";
import {
	hotspotSubjectDisplay,
	navigationForMode,
	routeErrorMessage,
	routeUnavailableMessage,
	traceEventSubjectDisplay,
} from "@/shared/lib/dashboard-mode";

describe("navigationForMode", () => {
	it("shows Proposals in live mode only when read capability exists", () => {
		expect(navigationForMode("live", true).map((item) => item.label)).toEqual([
			"Hotspots",
			"Signals",
			"Sessions",
			"Proposals",
			"Events",
			"Routes",
			"About",
		]);
		expect(
			navigationForMode("live", false).map((item) => item.label),
		).not.toContain("Proposals");
	});

	it("keeps existing local navigation in local mode", () => {
		expect(navigationForMode("local").map((item) => item.label)).toEqual([
			"Hotspots",
			"Sessions",
			"Proposals",
			"Events",
			"Routes",
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

describe("traceEventSubjectDisplay", () => {
	it("preserves raw server subjects in live mode", () => {
		expect(
			traceEventSubjectDisplay(
				{ subject: "/srv/repo/src/main.rs", subjectKind: "file" },
				{ mode: "live", repositoryPath: "/srv/repo" },
			).label,
		).toBe("/srv/repo/src/main.rs");
	});
});

describe("routeUnavailableMessage", () => {
	it("returns a live-mode explanation when proposal APIs are not configured", () => {
		expect(routeUnavailableMessage("proposals", "live")).toContain(
			"not configured",
		);
		expect(routeUnavailableMessage("proposal-detail", "live")).toContain(
			"not configured",
		);
		expect(routeUnavailableMessage("routes", "live")).toBeNull();
	});

	it("keeps live-capable routes available", () => {
		expect(routeUnavailableMessage("proposals", "live", true)).toBeNull();
		expect(routeUnavailableMessage("proposal-detail", "live", true)).toBeNull();
		expect(routeUnavailableMessage("hotspots", "live")).toBeNull();
		expect(routeUnavailableMessage("signals", "live")).toBeNull();
		expect(routeUnavailableMessage("sessions", "live")).toBeNull();
		expect(routeUnavailableMessage("session-detail", "live")).toBeNull();
		expect(routeUnavailableMessage("events", "live")).toBeNull();
		expect(routeUnavailableMessage("routes", "live")).toBeNull();
	});
});

describe("routeErrorMessage", () => {
	it("explains missing live publication", () => {
		expect(routeErrorMessage("live", 404, "missing")).toContain(
			"scryrs route publish <PATH>",
		);
	});

	it("distinguishes live upstream failure from local artifact corruption", () => {
		expect(routeErrorMessage("live", 502, "failed")).toContain(
			"Live route service",
		);
		expect(routeErrorMessage("local", 502, "failed")).toContain("regenerate");
	});
});
