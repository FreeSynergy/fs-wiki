# Label - Usage

!!! note
    The examples are kept syntax-neutral. As soon as the implementation moves to
    Rust and Blinc, the `usage.md` examples will be replaced in a separate pass.

## Captioning a field

```jsx
<Label for="username">Username</Label>
<TextInput id="username" />
```

## Required field

```jsx
<Label for="email" required={true}>Email</Label>
```

## Access key

```jsx
<Label for="save" shortcut="S">Save</Label>
```

## Hidden label for screen readers

```jsx
<SearchInput>
  <Label hidden={true}>Enter a search term</Label>
</SearchInput>
```

## Scenarios

| Scenario | Recommendation |
|----------|----------------|
| Form field | label together with [field](../../fields/field/) |
| Icon-only button | `aria-label` on the button, no label element |
| Group of fields | a visible group heading, no individual labels |
| Placeholder as a replacement | not allowed |
