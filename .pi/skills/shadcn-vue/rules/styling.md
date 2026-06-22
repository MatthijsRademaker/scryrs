# Styling and Customization

## Use Semantic Colors

```vue
<div class="bg-primary text-primary-foreground">
  <p class="text-muted-foreground">Secondary text</p>
</div>
```

Do not use raw color utility values for normal dashboard styling.

## Use Built-in Variants First

```vue
<Button variant="outline">Inspect</Button>
```

Prefer variant props before ad hoc class overrides.

## `class` Is for Layout

Use `class` for layout such as `max-w-md`, `mx-auto`, `grid`, `gap-4`, and similar placement concerns. Do not use it to bypass component color system.

## Use `gap-*`, Not `space-x-*` or `space-y-*`

```vue
<div class="flex flex-col gap-4">
  <Input />
  <Input />
</div>
```

## Use `size-*` When Width and Height Match

Prefer `size-10` over paired width and height utilities.

## Use `truncate`

Prefer `truncate` over long overflow class combinations.

## No Manual `dark:` Color Overrides

Use semantic tokens. Light and dark styling should come from token values in `src/app/styles.css`.

## Use `cn()` for Conditional Merge-Sensitive Classes

```vue
<script setup lang="ts">
import { cn } from '@/shared/lib/utils'
</script>

<template>
  <div :class="cn('flex items-center', active ? 'bg-primary text-primary-foreground' : 'bg-muted')" />
</template>
```

## No Manual Overlay z-index

Dialog, Sheet, Drawer, DropdownMenu, Popover, Tooltip, and HoverCard manage their own stacking.
