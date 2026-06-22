# Component Composition

## Grouped Items Stay Inside Group Primitive

```vue
<SelectContent>
  <SelectGroup>
    <SelectItem value="file">File</SelectItem>
    <SelectItem value="search">Search</SelectItem>
  </SelectGroup>
</SelectContent>
```

Apply same rule to dropdown, menubar, context-menu, and command primitives.

## Use Existing Feedback Components

- callouts use `Alert`
- empty states use `Empty`
- separators use `Separator`
- loading placeholders use `Skeleton`
- state markers use `Badge`

## Overlay Components Need Title Primitive

`DialogTitle`, `SheetTitle`, and `DrawerTitle` are required. Use `class="sr-only"` when title should stay visually hidden.

## Use Full Card Composition

```vue
<Card>
  <CardHeader>
    <CardTitle>Hotspots</CardTitle>
    <CardDescription>Ranked by score.</CardDescription>
  </CardHeader>
  <CardContent>...</CardContent>
  <CardFooter>...</CardFooter>
</Card>
```

## Button Pending State Is Composed

```vue
<Button disabled>
  <Spinner data-icon="inline-start" />
  Loading
</Button>
```

Do not invent `isLoading` or `isPending` prop for Button.

## TabsTrigger Lives Inside TabsList

```vue
<Tabs default-value="hotspots">
  <TabsList>
    <TabsTrigger value="hotspots">Hotspots</TabsTrigger>
    <TabsTrigger value="sessions">Sessions</TabsTrigger>
  </TabsList>
</Tabs>
```

## Avatar Needs Fallback

```vue
<Avatar>
  <AvatarImage src="/avatar.png" alt="Operator" />
  <AvatarFallback>OP</AvatarFallback>
</Avatar>
```
