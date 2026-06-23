<script setup lang="ts">
import { computed, ref } from "vue";
import { useRoute } from "vue-router";
import { IconActivity, IconFlame, IconInfo, IconListTree } from "@/shared/ui";

const navItems = [
  { to: "/", label: "Hotspots", icon: IconFlame, match: ["hotspots", "subject-detail"] },
  { to: "/sessions", label: "Sessions", icon: IconListTree, match: ["sessions", "session-detail"] },
  { to: "/events", label: "Events", icon: IconActivity, match: ["events"] },
  { to: "/about", label: "About", icon: IconInfo, match: ["about"] },
];

const route = useRoute();
const isActive = (match: string[]) => match.includes(String(route.name));

// Graceful degradation: if the brand asset is missing/fails, fall back to a wordmark.
const logoFailed = ref(false);
const logoSrc = "/brand/logo.png";
const activeLabel = computed(() => navItems.find((item) => isActive(item.match))?.label ?? "");
</script>

<template>
  <div class="relative min-h-screen bg-background text-foreground">
    <!-- Single ambient aurora/lens glow behind all content -->
    <div class="pointer-events-none fixed inset-0 -z-10 aurora"></div>

    <aside class="glass-surface fixed inset-y-0 left-0 z-20 hidden w-64 flex-col rounded-none border-y-0 border-l-0 text-sidebar-foreground md:flex">
      <div class="border-b border-border p-5">
        <RouterLink to="/" class="block no-underline">
          <img
            v-if="!logoFailed"
            :src="logoSrc"
            alt="scryrs"
            class="mx-auto h-24 w-24 rounded-xl object-contain"
            @error="logoFailed = true"
          />
          <div v-else class="flex flex-col items-center gap-1 py-3">
            <span class="text-2xl font-semibold tracking-[0.3em] text-foreground">scryrs</span>
            <span aria-hidden="true" class="text-primary drop-shadow-[0_0_8px_var(--glow-accent)]">◆</span>
          </div>
        </RouterLink>
        <p class="mt-3 text-center text-[0.7rem] font-medium uppercase tracking-[0.25em] text-muted-foreground">
          Trace Dashboard
        </p>
      </div>

      <nav class="flex flex-1 flex-col gap-1 p-3">
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="group relative flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm no-underline transition-[color,background-color] duration-200 ease-out"
          :class="
            isActive(item.match)
              ? 'bg-primary/10 text-foreground'
              : 'text-muted-foreground hover:bg-foreground/5 hover:text-foreground'
          "
        >
          <span
            v-if="isActive(item.match)"
            class="absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-full bg-primary shadow-[0_0_10px_2px_var(--glow-accent)]"
          ></span>
          <component
            :is="item.icon"
            class="size-4 shrink-0"
            :class="isActive(item.match) ? 'text-primary' : 'text-muted-foreground group-hover:text-foreground'"
          />
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>

      <div class="border-t border-border p-4 text-xs text-muted-foreground">
        Local-only viewer for .scryrs artifacts.
      </div>
    </aside>

    <div class="md:pl-64">
      <header class="glass-surface sticky top-0 z-10 rounded-none border-x-0 border-t-0 px-4 py-3 md:hidden">
        <nav class="flex gap-2 overflow-auto">
          <RouterLink
            v-for="item in navItems"
            :key="item.to"
            :to="item.to"
            class="rounded-lg px-3 py-2 text-sm no-underline transition-colors duration-200"
            :class="
              isActive(item.match)
                ? 'bg-primary/15 text-primary'
                : 'text-muted-foreground hover:text-foreground'
            "
          >
            {{ item.label }}
          </RouterLink>
        </nav>
      </header>
      <main class="mx-auto flex max-w-7xl flex-col gap-6 p-4 md:p-8">
        <RouterView v-slot="{ Component }">
          <Transition
            mode="out-in"
            enter-active-class="transition duration-200 ease-out"
            enter-from-class="opacity-0 translate-y-1"
            leave-active-class="transition duration-150 ease-in"
            leave-to-class="opacity-0"
          >
            <component :is="Component" :key="activeLabel || route.fullPath" />
          </Transition>
        </RouterView>
      </main>
    </div>
  </div>
</template>
