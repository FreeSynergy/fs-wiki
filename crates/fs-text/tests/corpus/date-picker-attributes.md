# Date Picker - Attributes

Only **own** attributes. Everything else comes from the base class, see
[base](../../../primitives/basics/base.md).

## Main attributes

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `value` | date | `null` | selected date |
| `rangeStart` | date | `null` | start of the range |
| `rangeEnd` | date | `null` | end of the range |
| `entries` | array[] | `[]` | entries for the visible window (supplied by the caller) |

## Behaviour

| Attribute | Type | Default | Description |
|-----------|---------|---------|-------------|
| `min` | date | `null` | earliest selectable date |
| `max` | date | `null` | latest selectable date |
| `firstDayOfWeek` | number | `1` | Monday = 1, Sunday = 0 |
| `quickPick` | array[] | `[heute, morgen]` | quick selection below the grid |
| `closeOnSelect` | boolean | `true` | close after the selection |

## Events

| Attribute | Type | Description |
|-----------|------|-------------|
| `onChange` | function | date or range changed |
| `onMonthChange` | function | shown month changed, reloads the window |
| `onEntriesLoad` | function | the entries of a window are loaded |
| `onEntryClick` | function | opened an entry in the day list |
| `onEntryCreate` | function | saved an entry from the quick entry |
| `onEntryUpdate` | function | changed an entry from the day list |
| `onEntryMove` | function | dragged an entry onto another day |
| `onEntryDelete` | function | deleted an entry from the day list |

## Accessibility

| Attribute | Type | Description |
|-----------|------|-------------|
| `role` | string | `dialog` with `grid` in the grid, cells `gridcell` |
| `aria-selected` | boolean | marks the selected day |
| `label` | string | mandatory, e.g. "Appointment" |
| `aria-label` (day) | string | the day names the date and the entries, e.g. "14 October, 3 entries" |
| `role` (day list) | string | `list` with `listitem`, entries additionally `button` |
| `aria-busy` | boolean | `true` while the window is being reloaded |

## Data connection

Without `sources` the picker is a plain calendar. With `sources` it reads the
data set through the pattern in [data API](../../../../data/api.md).

| Attribute | Type | Default | Description |
|-----------|---------|---------|-------------|
| `sources` | array[] | `[]` | models with a date reference, each as a name (`events`) or as an object (`resource`, `dateField`, `filter`) |
| `dateField` | string | `start` | field that supplies the day: `start`, `dueAt`, `createdAt`, …; overridable per source |
| `endField` | string | `end` | field for the end, for multi-day entries |
| `titleField` | string | `title` | field for the title in the day list |
| `typeField` | string | `type` | field for assigning an entry type |
| `filter` | string | `null` | filter by the API pattern, e.g. `status:eq:open;assigneeId:eq:user_01HZXA` |
| `scopeId` | reference | `null` | narrow down to one calendar, one board or one project |
| `loadWindow` | number | `1` | months beyond the visible window that are loaded along with it |
| `entryLimit` | number | `4` | entries per day, then "+ n more" |
| `showEntries` | enum | `beides` | `aus`, `punkte`, `liste`, `beides` |
| `autoReload` | boolean | `true` | follows `updatedSince` or the webhooks of the loaded window |

## Writing

| Attribute | Type | Default | Description |
|-----------|---------|---------|-------------|
| `creatable` | boolean | `false` | offer the quick entry in the overlay (`POST`) |
| `createFields` | array[] | `[title, start, end]` | fields of the quick entry, one mandatory field is enough |
| `movable` | boolean | `false` | move an entry by dragging it onto another day (`PATCH`) |
| `editable` | boolean | `false` | edit an entry from the day list |
| `deletable` | boolean | `false` | delete an entry from the day list (soft delete) |
| `confirmDelete` | boolean | `true` | show an [alert-dialog](../../overlays/alert-dialog/) before deleting |

## Entry object

An entry in `entries` follows the [standard fields](../../../../data/fields.md).

| Attribute | Type | Description |
|-----------|------|-------------|
| `id` | reference | ID of the record in the source |
| `title` | string | title in the day list |
| `start` | datetime | start; the value comes from `dateField` |
| `end` | datetime | end; when missing, one day applies |
| `allDay` | boolean | appointment without a time |
| `type` | enum | `event`, `task`, `document`, … — decides the marker |
| `color` | string \| null | own colour accent, otherwise by `type` |
| `done` | boolean | done (tasks), muted display |
| `recurrenceRule` | string \| null | RRULE after RFC 5545, series with a break at the dot |

## Inherited from

`id`, `visible`, `enabled`, `size`, `theme`, `rounded`, `state`, `tabIndex`,
`onClick`, `onFocus`, `onBlur`, `animation`, `gesture`, `aria-*` →
[Base component](../../../primitives/basics/base.md)
