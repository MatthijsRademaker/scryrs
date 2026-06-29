import { createRouter, createWebHistory } from "vue-router";

export const router = createRouter({
	history: createWebHistory(),
	routes: [
		{
			path: "/",
			name: "hotspots",
			component: () => import("@/views/HotspotsView.vue"),
		},
		{
			path: "/subjects/:subjectKind/:subject",
			name: "subject-detail",
			component: () => import("@/views/SubjectDetailView.vue"),
		},
		{
			path: "/sessions",
			name: "sessions",
			component: () => import("@/views/SessionsView.vue"),
		},
		{
			path: "/sessions/:sessionId",
			name: "session-detail",
			component: () => import("@/views/SessionDetailView.vue"),
		},
		{
			path: "/events",
			name: "events",
			component: () => import("@/views/EventsView.vue"),
		},
		{
			path: "/signals",
			name: "signals",
			component: () => import("@/views/SignalsView.vue"),
		},
		{
			path: "/about",
			name: "about",
			component: () => import("@/views/AboutView.vue"),
		},
		{
			path: "/:pathMatch(.*)*",
			name: "not-found",
			component: () => import("@/views/NotFoundView.vue"),
		},
	],
});
