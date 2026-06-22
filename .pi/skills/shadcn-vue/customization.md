# Customization and Theming

Dashboard components rely on semantic CSS variable tokens. Change tokens in one place and component styling updates everywhere.

## Global CSS File

Keep dashboard-wide tokens in:

```text
crates/scryrs-dashboard/frontend/src/app/styles.css
```

Do not create extra global theme files for ordinary dashboard work.

## Token Model

Use CSS variables for semantic surfaces and states:

- `--background`, `--foreground`
- `--card`, `--card-foreground`
- `--primary`, `--primary-foreground`
- `--secondary`, `--secondary-foreground`
- `--muted`, `--muted-foreground`
- `--accent`, `--accent-foreground`
- `--destructive`, `--destructive-foreground`
- `--border`, `--input`, `--ring`
- `--chart-1` through `--chart-5`
- dashboard-specific tokens such as `--event-file`, `--event-search`, `--event-edit` when needed

## Tailwind CSS v4 Registration

Register project tokens through `@theme inline` in `src/app/styles.css`:

```css
:root {
  --warning: oklch(0.84 0.16 84);
  --warning-foreground: oklch(0.28 0.07 46);
}

.dark {
  --warning: oklch(0.41 0.11 46);
  --warning-foreground: oklch(0.99 0.02 95);
}

@theme inline {
  --color-warning: var(--warning);
  --color-warning-foreground: var(--warning-foreground);
}
```

## Preferred Customization Order

1. Built-in component variants
2. semantic token classes
3. token additions in `src/app/styles.css`
4. thin wrapper components over shadcn-vue primitives

## Good Patterns

```vue
<Button variant="outline">Inspect</Button>

<Card class="mx-auto max-w-md">
  <CardContent>Dashboard</CardContent>
</Card>

<div class="bg-warning text-warning-foreground">Slow query</div>
```

Avoid raw color utility values for normal dashboard styling.
