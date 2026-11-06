# Data formats

Formats define **how** data is stored and exchanged between programs. The models in [models.md](models.md) are format-independent.

## Overview

| Format | Type | Usage | Short example |
|--------|------|-------|---------------|
| **JSON** | structured | API responses, config, export | `{"id":"doc_1","title":"Wiki"}` |
| **NDJSON** | line-based | streams, bulk export, logs | `{"id":"msg_1"}\n{"id":"msg_2"}` |
| **YAML** | structured + comments | config files (Homer, Homepage) | `services:\n  - name: Wiki` |
| **TOML** | structured + comments | project and tool config | `[app]\nname = "Homarr"` |
| **CSV/TSV** | tabular | import/export of lists | `id,title\n1,Wiki` |
| **XML** | structured | legacy systems, RSS/Atom | `<item><title>Wiki</title></item>` |
| **MessagePack** | binary | internal compact transfer | `{"id":"doc_1"}` (binary) |
| **Parquet** | binary, columnar | analytics, large exports | file export from `chart` |
| **Markdown + frontmatter** | text + metadata | wiki content, documentation | `---\ntitle: Wiki\n---` |
| **iCalendar (ICS)** | calendar standard | appointments, series | `BEGIN:VEVENT` |
| **vCard (VCF)** | contact data standard | contacts | `BEGIN:VCARD` |
| **GeoJSON** | geodata | maps, places | `{"type":"Point","coordinates":[13.4,52.5]}` |
| **JSON Schema** | metadata | validating models | `{"type":"object","required":["id"]}` |

## JSON

Standard format for all API communication.

```json
{
  "id": "doc_01HZX9",
  "title": "FreeSynergy overview",
  "status": "active",
  "createdAt": "2026-10-01T09:00:00+02:00",
  "tags": ["docs", "wiki"],
  "author": { "id": "user_01HZXA", "displayName": "Anna Weber" }
}
```

Rules:

- UTF-8, no BOM
- keys in `camelCase`
- no duplicate keys, no comments
- numbers without leading zeros, no `NaN`/`Infinity`

## NDJSON (JSON Lines)

One JSON document per line — for streams and bulk import.

```json
{"id":"msg_01","channelId":"chan_01","body":"Hello"}
{"id":"msg_02","channelId":"chan_01","body":"How are you?"}
```

## YAML

For configuration files (e.g. Homer `config.yml`, Homepage `services.yaml`).

```yaml
# dashboard configuration
services:
  - name: Wiki
    url: https://wiki.example.org
    icon: mediawiki.svg
  - name: Calendar
    url: https://cal.example.org
    widget:
      type: calendar
      url: https://cal.example.org/dav
```

## TOML

```toml
[app]
name = "Homarr"
port = 7575

[auth]
provider = "oidc"
```

## CSV / TSV

First line = header, separator `,` (CSV) or `\t` (TSV).

```csv
id,displayName,email,status
user_01,Anna Weber,anna@example.org,active
user_02,"Weber, Anna",anna2@example.org,invited
```

Rules:

- quote separators and line breaks in values with `"`
- header line is mandatory, `camelCase` column names
- character set UTF-8 (BOM only for Excel compatibility)

## Markdown + frontmatter

Standard for documents and for this documentation.

```markdown
---
title: Standardised data
lang: en
tags: [data, standard]
updatedAt: 2026-10-31
---

# Standardised data
```

## iCalendar (ICS)

```ics
BEGIN:VEVENT
UID:evt_01HZXB@example.org
DTSTART:20261105T100000Z
DTEND:20261105T110000Z
SUMMARY:Team sync
RRULE:FREQ=WEEKLY;BYDAY=WE
END:VEVENT
```

## vCard (VCF)

```vcf
BEGIN:VCARD
VERSION:3.0
N:Weber;Anna;;;
FN:Anna Weber
EMAIL;TYPE=WORK:anna@example.org
END:VCARD
```

## GeoJSON

```json
{
  "type": "Feature",
  "geometry": { "type": "Point", "coordinates": [13.404954, 52.520008] },
  "properties": { "id": "loc_01", "name": "Hauptbahnhof Berlin" }
}
```

## JSON Schema (validation)

```json
{
  "$id": "https://freesynergy.github.io/schema/task.json",
  "type": "object",
  "required": ["id", "title", "status", "createdAt", "updatedAt"],
  "properties": {
    "id": { "type": "string" },
    "title": { "type": "string", "maxLength": 200 },
    "status": { "enum": ["open", "in_progress", "done", "archived"] },
    "dueAt": { "type": ["string", "null"], "format": "date-time" }
  }
}
```

## Which format for what?

| Goal | Format | Reason |
|------|--------|--------|
| API exchange | JSON | ubiquitous, typed and validatable |
| config with comments | YAML or TOML | readable, comments allowed |
| static dashboards | YAML | no database needed (Homer, Homepage) |
| bulk export / import | NDJSON | line-wise, streamable |
| table import (foreign systems) | CSV/TSV | widely supported |
| calendar exchange | ICS | standard, CalDAV compatible |
| contact exchange | VCF | standard, CardDAV compatible |
| maps/places | GeoJSON | usable directly in `map-view` |
| analytics export | Parquet | columnar, compact |
| content/documents | Markdown + frontmatter | diffable, versionable |

## Encoding and escaping

- **character set**: UTF-8 always, without BOM (exception: CSV for Excel)
- **line break**: `\n` in files, `\r\n` only in ICS/vCard per the standard
- **escaping**: HTML is not stored in `markdown` fields, it is produced when rendering
- **date/time**: ISO 8601 with offset, time without localisation
- **numbers**: dot as decimal separator, thousands separators only in the display

## See also

- [standard models](models.md)
- [standard fields](fields.md)
- [conventions](conventions.md)
- [data API](api.md)
