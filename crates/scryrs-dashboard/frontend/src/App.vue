<script setup lang="ts">
import { MotionConfig } from "motion-v";
import DashboardShell from "@/shared/ui/shell/DashboardShell.vue";
</script>

<template>
  <!--
    reducedMotion="user" defers all motion-v animation to the OS
    prefers-reduced-motion setting — the same signal the global CSS
    kill-switch in app/styles.css keys off. Single source of truth; no
    competing second system.
  -->
  <MotionConfig :reduced-motion="'user'">
    <DashboardShell />
  </MotionConfig>
</template>
