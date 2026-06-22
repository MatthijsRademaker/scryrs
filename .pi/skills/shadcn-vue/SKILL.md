---
name: shadcn-vue
description: Dashboard UI guidance for `crates/scryrs-dashboard/frontend/`. Use before adding, changing, or debugging shadcn-vue components, `components.json`, shared UI primitives, semantic tokens, or shadcn-vue CLI usage.
---

# shadcn-vue Dashboard Guidance

Read this skill before any dashboard UI change in `crates/scryrs-dashboard/frontend/`.

## Project Context

- Frontend path: `crates/scryrs-dashboard/frontend/`
- Package manager: **Bun only**
- CLI runner: `bunx --bun shadcn-vue@latest`
- UI source path: `src/shared/ui/`
- Utils path: `src/shared/lib/utils.ts`
- Global styles path: `src/app/styles.css`
- Stack: Vue 3, Vite, Tailwind CSS v4, shadcn-vue, Reka UI, lucide-vue, Vue Router, Pinia

## Core Rules

1. **Use existing components first.** Check `src/shared/ui/` before building fresh markup.
2. **Compose, do not reinvent.** Dashboard views should be built from Sidebar, Card, Table, Badge, Button, Tabs, Select, Tooltip, Skeleton, Alert, Empty, Sheet, and Dialog primitives.
3. **Use semantic tokens.** Prefer `bg-background`, `text-muted-foreground`, `border-border`, `bg-destructive`, and project token aliases in `src/app/styles.css`.
4. **Use Bun-only commands.** No alternate package runners for dashboard frontend work.
5. **Read component docs first.** Run `bunx --bun shadcn-vue@latest docs <component>` before adding or adjusting a component you are unsure about.

## Critical Rules

### Styling → [rules/styling.md](./rules/styling.md)

- `class` for layout, not for color-system overrides
- no `space-x-*` or `space-y-*`; use `gap-*`
- use `size-*` when width and height match
- use `truncate` shorthand
- no manual `dark:` color overrides
- use `cn()` for conditional merge-sensitive classes
- no manual overlay z-index

### Forms → [rules/forms.md](./rules/forms.md)

- forms use `FieldGroup` + `Field`
- `InputGroup` uses `InputGroupInput` or `InputGroupTextarea`
- grouped checkboxes or radios use `FieldSet` + `FieldLegend`
- short option sets use `ToggleGroup`
- validation uses `data-invalid` on `Field` and `aria-invalid` on control

### Composition → [rules/composition.md](./rules/composition.md)

- grouped menu and select items stay inside matching group primitive
- Dialog, Sheet, and Drawer always include title primitive
- use full Card composition
- Button loading state is composed with `Spinner` + `disabled`
- use `Alert`, `Empty`, `Separator`, `Skeleton`, and `Badge` instead of ad hoc markup

### Icons → [rules/icons.md](./rules/icons.md)

- import icons from configured library
- icons in `Button` use `data-icon="inline-start"` or `data-icon="inline-end"`
- avoid manual sizing classes inside shadcn-vue components
- pass icon component objects, not string keys

## Workflow

1. Read `components.json`, `src/app/styles.css`, and `src/shared/ui/` before editing UI.
2. Run `bunx --bun shadcn-vue@latest info` in `crates/scryrs-dashboard/frontend/` when you need current aliases or installed component list.
3. Run `bunx --bun shadcn-vue@latest search` before writing custom UI for common primitives.
4. Run `bunx --bun shadcn-vue@latest docs <component>` before adding or changing unfamiliar component usage.
5. Add missing components with `bunx --bun shadcn-vue@latest add <component>`.
6. Review generated files for imports, dependencies, and rule compliance before continuing.

## Quick Reference

```bash
cd crates/scryrs-dashboard/frontend

# inspect project config
bunx --bun shadcn-vue@latest info

# find components
bunx --bun shadcn-vue@latest search @shadcn -q "sidebar"

# read docs
bunx --bun shadcn-vue@latest docs button dialog select table sidebar

# add components
bunx --bun shadcn-vue@latest add button card dialog table sidebar select
```

## Related Files

- [cli.md](./cli.md) — Bun-only shadcn-vue CLI reference
- [customization.md](./customization.md) — semantic token and theming rules for `src/app/styles.css`
- [rules/styling.md](./rules/styling.md)
- [rules/forms.md](./rules/forms.md)
- [rules/composition.md](./rules/composition.md)
- [rules/icons.md](./rules/icons.md)
