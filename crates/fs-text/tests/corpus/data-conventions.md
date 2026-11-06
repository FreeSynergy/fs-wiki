# Conventions

Conventions keep documentation and data consistent across all categories.

## Naming

| Context | Style | Example |
|---------|-------|---------|
| fields and attributes | `camelCase` | `createdAt`, `displayName` |
| JSON keys in configs | `kebab-case` or `camelCase` (consistent per file) | `tap-threshold` / `tapThreshold` |
| directory and file names | `kebab-case` | `long-press/`, `fade-in/` |
| categories | lowercase, the customary name | `projectmanagement`, `whiteboards` |
| program directories | official project name, lowercase | `homarr`, `mediawiki`, `xwiki` |
| constants and environment variables | `SCREAMING_SNAKE_CASE` | `FS_API_TOKEN` |
| enums | lowercase, singular, `_` for spaces | `in_progress` |
| events | `<resource>.<action>` lowercase | `document.published` |

## File conventions

### Programs (frontends)

Every program in `apps/<category>/frontends/<program>/` gets **seven** files:

| File | Content |
|------|---------|
| `index.md` | overview + links to the other six files |
| `description.md` | what the program does, advantages, differentiation, technical details |
| `features.md` | features, widgets/plugins, what it does NOT do |
| `menus.md` | menu items, context menus, role menus |
| `layout.md` | window structure (ASCII), UI/UX, accessibility, performance |
| `shortcuts.md` | all keyboard shortcuts (always separate!) |
| `api.md` | API type, auth, endpoints, examples, rate limits, SDKs |

### Components

`components/<band>/<group>/<element>/` with `description.md`, `appearance.md`, `attributes.md` and `usage.md`. Bands = levels: `primitives`, `widgets`, `workbenches`.

### Gestures

`gestures/<category>/<gesture>/` with `description.md`, `attributes.md`, `appearance.md`
(categories: `tap`, `swipe`, `transform`, `special`, `device`, `combined`).

### Animations

`animations/<animation>/` with `description.md`, `attributes.md`, `appearance.md`.

### Data

Cross-cutting files directly in `data/`: `index.md`, `standards.md`, `mappings.md`, `formats.md`, `models.md`, `fields.md`, `conventions.md`, `api.md`.

Below that the rule is: **data domain → standard → files**:

```
data/<domain>/index.md                       overview + table of the domain's standards
data/<domain>/<standard>/description.md      general description (what, scope, limits)
data/<domain>/<standard>/storage.md          how data in it is stored (DB, files, patches)
data/<domain>/<standard>/examples.md         optional, when needed
data/<domain>/<standard>/mapping.md          optional, our fields ↔ standard fields
```

- domain directories: lowercase, ASCII (`contacts`, `identity`, `geodata`)
- standard directories: official short name, lowercase (`jscontact`, `jscalendar`, `vcard`)
- every entry ends, as everywhere, with `## See also`

## Links

- cross-references within a program: relative (`[api.md](api.md)`)
- references across areas: relative with `../` (from `data/`: `[Gestures](../interactions/gestures/)`)
- category index always as a directory link (from `data/`: `[Messenger](../programs/apps/messenger/)`)
- every file ends with a `## See also` section
- no links to files that do not exist — exception: cross-references
  within one's own language version (rule 6 of the language convention)

## Language convention

1. German (`docs/de/`) is the lead version and is written first
2. English (`docs/en/`) mirrors the directory structure (French paused)
3. directory and file names are language-independent (same schema in `de/` and `en/`)
4. product and project names stay untranslated
5. technical terms from UI and API stay English (`Modal`, `Toast`, `Endpoint`)
6. **never link across languages:** in `en/` (and later languages) every
   cross-reference points at one's own language version (`en → en`), never at
   `de/` — even if the target file does not exist there yet; the translation
   will follow. Language switching happens only through the language switcher
   in `index.md`

## Versioning and compatibility

| Rule | Description |
|------|-------------|
| SemVer | `MAJOR.MINOR.PATCH` for schemas and API versions |
| additive changes | new fields are `MINOR`, no breaking changes |
| breaking change | removing/renaming a field or changing its type is `MAJOR` |
| API version in the path | `/api/v1/…`; old versions for at least 12 months |
| deprecation | field stays marked with `deprecated: true`, for at least 2 minor releases |
| changelog | document changes to models in `data/` |

## JSON schema pattern

Standard schema layout for every model:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://freesynergy.github.io/schema/contact.json",
  "title": "Contact",
  "type": "object",
  "additionalProperties": true,
  "required": ["id", "firstName", "lastName", "createdAt", "updatedAt"],
  "properties": {
    "id": { "type": "string", "pattern": "^con_[0-9A-HJKMNP-TV-Z]{26}$" },
    "firstName": { "type": "string", "maxLength": 100 },
    "lastName": { "type": "string", "maxLength": 100 },
    "emails": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["value"],
        "properties": {
          "type": { "enum": ["work", "home", "other"] },
          "value": { "type": "string", "format": "email" }
        }
      }
    }
  }
}
```

Example document for it:

```json
{
  "id": "con_01HZXH7Q3T8V2B4C6D9E0F1A2K",
  "firstName": "Anna",
  "lastName": "Weber",
  "emails": [{ "type": "work", "value": "anna@example.org" }],
  "createdAt": "2026-10-01T09:00:00+02:00",
  "updatedAt": "2026-10-12T14:05:00+02:00"
}
```

## Validation rules

| Rule | Description |
|------|-------------|
| required fields | `required` in the schema; a missing required field = `400 validation_error` |
| null values | optional fields may be `null`, required fields may not |
| enums | only documented values; unknown values = `422` |
| lengths | document `maxLength` and make it visible in the UI |
| references | must exist or be `null`; dead references are cleaned up |
| duplicates | `slug`, `email`, `username` are unique within the scope |
| time zones | `timezone` must be an IANA identifier |
| files | `mimeType` and `size` are required, `checksum` recommended |
| pagination | lists are never delivered unbounded; default `limit=25`, maximum `100` |

## Internationalisation

| Point | Convention |
|-------|------------|
| language codes | ISO 639-1: `de`, `en`, `fr` |
| regional codes | only where needed: `de-DE`, `fr-CH` |
| translation files | `locales/<code>.json` with `camelCase` keys |
| fallback | missing key → `de`, never show the key in the UI |
| date/time in the UI | format according to `locale`, data always ISO 8601 |
| numbers | data always with a dot; display according to `locale` |

```json
{
  "task.status.open": "Open",
  "task.status.in_progress": "In progress",
  "task.status.done": "Done"
}
```

## Accessibility in data

| Field | Required for | Description |
|-------|--------------|-------------|
| `ariaLabel` | all interactive elements | screen reader text |
| `role` | all interactive elements | ARIA role |
| `altText` | `image`, `avatar`, `icon` | alternative text |
| `respectReducedMotion` | all animations | reduce animations |
| `alternativeAction` | gestures | keyboard/button equivalent |
| `keyboardShortcut` | all actions | document in `shortcuts.md` |

## Quality assurance

- every new program documentation contains all seven files (no empty placeholders)
- short-form files (`index.md`) link to all the others
- `shortcuts.md` and `api.md` are never contained in other files
- missing features are named explicitly ("What X does NOT do")
- invented shortcuts or endpoints are a mistake — rather state "not present"
- **standard steward:** every field in `fields.md` names an evidence in [standards.md](standards.md) (standard plus number) or is expressly marked as an **own decision**. A field without evidence and without a marker is a mistake
- **no invented standard numbers:** an RFC or spec number is only named if it was looked up. Unverified statements are marked with ⚠️

## See also

- [data formats](formats.md)
- [data models](models.md)
- [standard fields](fields.md)
- [data API](api.md)
- [components](../components/)
