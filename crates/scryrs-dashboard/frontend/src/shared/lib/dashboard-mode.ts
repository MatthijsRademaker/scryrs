import type {
	DashboardMode,
	HotspotEntry,
	TraceEventItem,
} from "@/shared/api/client";
import { formatSubject, type SubjectDisplay } from "@/shared/lib/subject";

export interface DashboardNavItem {
	to: string;
	label: string;
	icon: "flame" | "activity" | "tree" | "inbox" | "info";
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
	{
		to: "/proposals",
		label: "Proposals",
		icon: "inbox",
		match: ["proposals", "proposal-detail"],
	},
	{ to: "/events", label: "Events", icon: "activity", match: ["events"] },
	{ to: "/routes", label: "Routes", icon: "activity", match: ["routes"] },
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
	{
		to: "/sessions",
		label: "Sessions",
		icon: "tree",
		match: ["sessions", "session-detail"],
	},
	{
		to: "/proposals",
		label: "Proposals",
		icon: "inbox",
		match: ["proposals", "proposal-detail"],
	},
	{ to: "/events", label: "Events", icon: "activity", match: ["events"] },
	{ to: "/routes", label: "Routes", icon: "activity", match: ["routes"] },
	{ to: "/about", label: "About", icon: "info", match: ["about"] },
];

export function navigationForMode(
	mode: DashboardMode | null | undefined,
	proposalReadsAvailable = false,
): DashboardNavItem[] {
	if (mode !== "live") return LOCAL_NAV;
	return proposalReadsAvailable
		? LIVE_NAV
		: LIVE_NAV.filter((item) => item.to !== "/proposals");
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

export function traceEventSubjectDisplay(
	event: Pick<TraceEventItem, "subject" | "subjectKind">,
	meta: { mode?: DashboardMode | null; repositoryPath?: string | null },
): SubjectDisplay {
	if (meta.mode === "live") {
		return {
			kind: "raw",
			label: event.subject ?? "lifecycle",
			isExternal: false,
			full: event.subject ?? "lifecycle",
		};
	}
	return formatSubject(event.subject, meta.repositoryPath, event.subjectKind);
}

export function routeUnavailableMessage(
	routeName: string,
	mode: DashboardMode | null | undefined,
	proposalReadsAvailable = false,
): string | null {
	if (
		mode === "live" &&
		!proposalReadsAvailable &&
		(routeName === "proposals" || routeName === "proposal-detail")
	) {
		return "Live proposal APIs are not configured for this dashboard.";
	}

	if (mode === "local" && routeName === "signals") {
		return "Signals are only available in live mode. Start the dashboard with --server-url and --repository-id to stream hotspot signals.";
	}

	return null;
}

export function routeErrorMessage(
	mode: DashboardMode | null | undefined,
	status: number,
	message: string,
): string {
	if (status === 404) {
		return mode === "live"
			? "No route manifest is published for this repository. Run `scryrs route publish <PATH>` with repository credentials, then retry."
			: "Route artifact not found. Run `scryrs route <PATH>` to generate the route manifest.";
	}
	if (status === 502) {
		return mode === "live"
			? "Live route service failed. Verify server availability and published manifest compatibility, then retry."
			: "Route artifact is malformed or incompatible. Run `scryrs route <PATH>` to regenerate it.";
	}
	return message;
}
