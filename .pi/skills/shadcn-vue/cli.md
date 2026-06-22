# shadcn-vue CLI Reference

Run all dashboard shadcn-vue CLI commands from `crates/scryrs-dashboard/frontend/` with **Bun only**:

```bash
bunx --bun shadcn-vue@latest <command>
```

Only use documented flags. Do not guess flags.

## Core Commands

### `info`

```bash
bunx --bun shadcn-vue@latest info
```

Use first. It shows framework, aliases, Tailwind version, icon library, and resolved paths from `components.json`.

### `docs`

```bash
bunx --bun shadcn-vue@latest docs button dialog select
```

Returns docs and example URLs. Read them before changing unfamiliar component usage.

### `search`

```bash
bunx --bun shadcn-vue@latest search @shadcn -q "sidebar"
```

Use before writing custom UI for common primitives.

### `add`

```bash
bunx --bun shadcn-vue@latest add button card table sidebar
```

Use to install missing components into `src/shared/ui/`. After generation, inspect files for imports and composition correctness.

### `view`

```bash
bunx --bun shadcn-vue@latest view @shadcn/button
```

Use when you need registry item details before installation.

### `init`

```bash
bunx --bun shadcn-vue@latest init --template vite
```

Use only for fresh dashboard scaffold work or full configuration reset.

### `apply`

```bash
bunx --bun shadcn-vue@latest apply <preset>
```

Use only when intentionally applying preset-driven config to existing project.

## Safe Update Flow

1. Inspect installed files first.
2. Use `docs` to confirm correct component API.
3. Use `add` to install missing component.
4. Read generated files.
5. Keep local scryrs-specific aliases, semantic tokens, and shell structure intact.

## Expected Dashboard Paths

- `components.json`
- `src/shared/ui/**`
- `src/shared/lib/utils.ts`
- `src/app/styles.css`

Keep generated imports aligned with those paths.
