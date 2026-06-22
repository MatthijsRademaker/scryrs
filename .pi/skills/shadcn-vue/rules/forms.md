# Forms and Inputs

## Form Layout Uses FieldGroup and Field

```vue
<FieldGroup>
  <Field>
    <FieldLabel for="session">Session</FieldLabel>
    <Select id="session" />
  </Field>
</FieldGroup>
```

Do not build form layout from plain `div` stacks when shadcn-vue field primitives fit.

## InputGroup Uses InputGroupInput or InputGroupTextarea

```vue
<InputGroup>
  <InputGroupInput placeholder="Filter events" />
</InputGroup>
```

## Buttons Inside Inputs Use InputGroupAddon

```vue
<InputGroup>
  <InputGroupInput placeholder="Search subject" />
  <InputGroupAddon>
    <Button size="icon">
      <SearchIcon data-icon="inline-start" />
    </Button>
  </InputGroupAddon>
</InputGroup>
```

## Short Option Sets Use ToggleGroup

```vue
<ToggleGroup spacing="2">
  <ToggleGroupItem value="all">All</ToggleGroupItem>
  <ToggleGroupItem value="file">File</ToggleGroupItem>
  <ToggleGroupItem value="search">Search</ToggleGroupItem>
</ToggleGroup>
```

## Group Related Controls with FieldSet and FieldLegend

```vue
<FieldSet>
  <FieldLegend variant="label">Visible columns</FieldLegend>
  <FieldGroup class="gap-3">
    <Field orientation="horizontal">
      <Checkbox id="score" />
      <FieldLabel for="score" class="font-normal">Score</FieldLabel>
    </Field>
  </FieldGroup>
</FieldSet>
```

## Validation Uses Field and Control Attributes Together

```vue
<Field data-invalid>
  <FieldLabel for="port">Port</FieldLabel>
  <Input id="port" aria-invalid />
</Field>
```

Use `data-disabled` on `Field` and `disabled` on control for disabled state.
