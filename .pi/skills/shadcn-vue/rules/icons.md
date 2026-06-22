# Icons

Use project icon library from `components.json`. Dashboard default is lucide.

## Icons in Button Use `data-icon`

```vue
<Button>
  <SearchIcon data-icon="inline-start" />
  Search
</Button>

<Button>
  Next
  <ArrowRightIcon data-icon="inline-end" />
</Button>
```

## Avoid Manual Sizing Classes Inside shadcn-vue Components

Do not add `size-4`, `w-4`, or `h-4` to icons inside `Button`, `DropdownMenuItem`, `Alert`, `Sidebar*`, and similar primitives unless custom sizing is intentional.

## Pass Icon Components, Not String Keys

```vue
<script setup lang="ts">
import { CheckIcon } from 'lucide-vue'
defineProps<{ icon: object }>()
</script>

<template>
  <component :is="icon" />
</template>

<!-- usage -->
<StatusBadge :icon="CheckIcon" />
```
