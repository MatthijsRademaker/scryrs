import type { DashboardMode, HotspotEntry } from "@/shared/api/client";
import { formatSubject, type SubjectDisplay } from "@/shared/lib/subject";

export interface DashboardNavItem {
	to: string;
	label: string;
	icon: "flame" | "activity" | "tree" | "info";
	match: string[];
}

const LOCAL_NAV: DashboardNavItem[] = [
	{
		to: "/",
		label: "Hotspots",
		icon: "flame",
		match: ["hotspots", "subject-detail"],
	},
	{
		to: "/sessions",
		label: "Sessions",
		icon: "tree",
		match: ["sessions", "session-detail"],
	},
	{ to: "/events", label: "Events", icon: "activity", match: ["events"] },
	{ to: "/about", label: "About", icon: "info", match: ["about"] },
];

const LIVE_NAV: DashboardNavItem[] = [
	{
		to: "/",
		label: "Hotspots",
		icon: "flame",
		match: ["hotspots", "subject-detail"],
	},
	{ to: "/signals", label: "Signals", icon: "activity", match: ["signals"] },
	{ to: "/about", label: "About", icon: "info", match: ["about"] },
];

export function navigationForMode(
	mode: DashboardMode | null | undefined,
): DashboardNavItem[] {
	return mode === "live" ? LIVE_NAV : LOCAL_NAV;
}

export function hotspotSubjectDisplay(
	entry: Pick<HotspotEntry, "subject" | "subjectKind">,
	meta: { mode?: DashboardMode | null; repositoryPath?: string | null },
): SubjectDisplay {
	if (meta.mode === "live") {
		return {
			kind: "raw",
			label: entry.subject,
			isExternal: false,
			full: entry.subject,
		};
	}
	return formatSubject(entry.subject, meta.repositoryPath, entry.subjectKind);
}

export function routeUnavailableMessage(
	routeName: string,
	mode: DashboardMode | null | undefined,
): string | null {
	if (mode === "live") {
		if (routeName === "sessions" || routeName === "session-detail") {
			return "Sessions are not available in live mode. This dashboard only proxies live hotspot rankings and signal streaming.";
		}
		if (routeName === "events") {
			return "Events are not available in live mode. Use Signals for replayed and live hotspot activity.";
		}
	}

	if (mode === "local" && routeName === "signals") {
		return "Signals are only available in live mode. Start the dashboard with --server-url and --repository-id to stream hotspot signals.";
	}

	return null;
}
