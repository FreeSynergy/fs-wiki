# Base component (base)

The **base component** is the common parent class of all FreeSynergy components.
It defines the attributes, states and sections that *every* component inherits.

!!! important "Decision (standardisation)"
    Since the standardisation, the `attributes.md` of a component lists **only its own
    attributes** plus the reference `Inherited from: base`. Everything written in this
    file is **not** repeated per component. Changes to the base class therefore take
    effect on all components automatically.

## Profile (required mask)

Every `description.md` begins with this table. It is deliberately not YAML frontmatter
(frontmatter was removed from this repo on purpose) but a Markdown table: it is readable
for humans **and** can be evaluated by `scripts/check_components.py`.

| Field | Meaning | Example (`search-field`) |
|-------|---------|--------------------------|
| **Category** | folder under `components/` | `fields` |
| **Inherits from** | parent classes, starting from `base` | `base`, `text-input` |
| **Variants** | variants on the same page | `simple`, `autocomplete`, `filter` |
| **States** | additional states compared to `base` | `loading`, `error` |
| **Gestures** | reference to [gestures](../../../interactions/gestures/) | `tap`, `type` |
| **Animations** | reference to [animations](../../../interactions/animations/) | `fade-in`, `expand` |
| **Related** | distinction and cross-references | `search-input`, `combobox` |
| **Status** | `documented` / `sketch only` / `missing` | `documented` |

## Inherited attributes

### Identity and visibility

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `id` | string | - | unique identifier |
| `label` | string | - | caption (see [label](label/)) |
| `visible` | boolean | `true` | visible? |
| `enabled` | boolean | `true` | operable? (`disabled` = the inverse) |

### Presentation

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `size` | enum | `medium` | `small`, `medium`, `large` (see size scale) |
| `theme` | enum | `default` | `default`, `dark` (follows the system colour palette) |
| `rounded` | boolean | `true` | rounded corners? |
| `variant` | enum | - | variant of the component, values defined per component |

### Size scale

| Size | Height | Icon | Font | Use |
|------|--------|------|------|-----|
| `small` | 28 px | 14 px | 13 px | dense surfaces, tables |
| `medium` | 36 px | 16 px | 14 px | standard |
| `large` | 44 px | 20 px | 16 px | touch, primary actions |

44 px is also the minimum hit area for touch operation.

### States (`state`)

Every component knows these state names. `appearance.md` describes what changes
visually per state; components may add states of their own.

| State | Trigger |
|-------|---------|
| `default` | resting state |
| `hover` | pointer over the element |
| `active` | pressed / selected |
| `focus` | keyboard or focus navigation |
| `disabled` | `enabled = false` |
| `loading` | the component is waiting for a response |
| `error` | input or action error |

### Interaction and events

| Attribute | Type | Description |
|-----------|------|-------------|
| `tabIndex` | number | order of keyboard navigation |
| `onClick` | function | triggered by pointer or keyboard |
| `onFocus` | function | gained focus |
| `onBlur` | function | lost focus |
| `gesture` | string[] | supported gestures (see [gestures](../../../interactions/gestures/)) |
| `animation` | string | transition when showing/hiding (see [animations](../../../interactions/animations/)) |

### Accessibility

| Attribute | Type | Description |
|-----------|------|-------------|
| `role` | string | ARIA role; the **value** is component-specific and named in the profile |
| `aria-label` | string | accessible name if there is no visible `label` |
| `aria-*` | - | state markings (`aria-disabled`, `aria-busy`, `aria-expanded`, …) |

## When a variant, when a component of its own?

| Criterion | → **variant** (`variant`/`type`) | → **own component** |
|-----------|----------------------------------|---------------------|
| Same data record, same events | yes | |
| Only appearance or docking changes | yes (`size`, `placement`, `orientation`) | |
| Own data structure needed (suggestion list, file stream, calendar) | | yes |
| Own lifetime or focus behaviour (overlay, modal) | | yes |
| Accessible role changes (`button` → `menu`) | | yes |
| Attribute overlap ≥ 75 % | yes | |

Real inheritance is expressed through the **Inherits from** field in the profile
(e.g. `context-menu` inherits from `menu`, `audio` inherits from `media-player`).

## File set per component

| File | Required | Content (the section order is fixed) |
|------|----------|--------------------------------------|
| `description.md` | yes | profile → description → purpose → variants → features → distinction → best practices → see also |
| `appearance.md` | yes | base layout → variants → sizes → colours and states → hover/focus → responsive |
| `attributes.md` | yes | main attributes → behaviour → presentation → events → accessibility → **inherited from** |
| `usage.md` | yes | application, scenarios and examples in one file |

## Naming convention

Directories English, lowercase with hyphens; section headings follow the language of the
document; attribute names `camelCase` English.

| Suffix | Meaning | Example |
|--------|---------|---------|
| `-input` | field with a data type | `text-input` |
| `-picker` | selection with an overlay or dialog | `date-picker` |
| `-editor` | editing content | `code-editor` |
| `-view` | data view | `list-view` |
| `-menu` / `-bar` | navigation bar | `context-menu`, `menubar` |

## See also

- [Basics – overview](./index.md)
- [Component overview](../index.md)
- [Gestures](../../../interactions/gestures/) and [Animations](../../../interactions/animations/)
- [Standardised data](../../widgets/data/) – same idea, only for fields and types
