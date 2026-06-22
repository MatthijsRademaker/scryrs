<script setup lang="ts">
import { Activity, Flame, Info, ListTree } from "lucide-vue";

const navItems = [
  { to: "/", label: "Hotspots", icon: Flame },
  { to: "/sessions", label: "Sessions", icon: ListTree },
  { to: "/events", label: "Events", icon: Activity },
  { to: "/about", label: "About", icon: Info },
];
</script>

<template>
  <div class="min-h-screen bg-background text-foreground">
    <aside class="fixed inset-y-0 left-0 hidden w-64 border-r bg-sidebar text-sidebar-foreground md:flex md:flex-col">
      <div class="border-b p-6">
        <p class="text-xs font-medium uppercase tracking-wider text-sidebar-foreground/70">scryrs</p>
        <h1 class="mt-2 text-xl font-semibold">Trace Dashboard</h1>
      </div>
      <nav class="flex flex-1 flex-col gap-2 p-4">
        <RouterLink v-for="item in navItems" :key="item.to" :to="item.to" class="flex items-center gap-3 rounded-md px-3 py-2 text-sm no-underline hover:bg-sidebar-accent hover:text-sidebar-accent-foreground" active-class="bg-sidebar-accent text-sidebar-accent-foreground">
          <component :is="item.icon" />
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>
      <div class="border-t p-4 text-xs text-sidebar-foreground/70">Local-only viewer for .scryrs artifacts.</div>
    </aside>
    <div class="md:pl-64">
      <header class="sticky top-0 z-10 border-b bg-background/95 px-4 py-3 backdrop-blur md:hidden">
        <nav class="flex gap-2 overflow-auto">
          <RouterLink v-for="item in navItems" :key="item.to" :to="item.to" class="rounded-md px-3 py-2 text-sm no-underline hover:bg-accent" active-class="bg-accent text-accent-foreground">{{ item.label }}</RouterLink>
        </nav>
      </header>
      <main class="mx-auto flex max-w-7xl flex-col gap-6 p-4 md:p-8">
        <RouterView />
      </main>
    </div>
  </div>
</template>
