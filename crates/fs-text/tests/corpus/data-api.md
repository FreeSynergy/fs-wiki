# Data API (standardised)

All programs expose their data using the same pattern so that workflows, imports and frontends work without special treatment. Concretely documented deviations live in the `api.md` of the respective program (`apps/<category>/frontends/<program>/api.md`).

## Overview

| Feature | Standard |
|---------|----------|
| style | REST, JSON |
| base | `https://<host>/api/v1` |
| formats | request and response `application/json` (bulk: `application/x-ndjson`) |
| methods | `GET`, `POST`, `PATCH`, `PUT`, `DELETE` |
| authentication | API key or bearer token, alternatively OAuth2 |
| encoding | UTF-8, JSON without comments |

## Resource pattern

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/v1/<resource>` | list with filter, sorting, pagination |
| `POST` | `/api/v1/<resource>` | create |
| `GET` | `/api/v1/<resource>/{id}` | load a single one |
| `PATCH` | `/api/v1/<resource>/{id}` | partial change |
| `PUT` | `/api/v1/<resource>/{id}` | replace completely |
| `DELETE` | `/api/v1/<resource>/{id}` | delete (soft delete where available) |
| `GET` | `/api/v1/<resource>/{id}/revisions` | revision history |
| `POST` | `/api/v1/<resource>/{id}/restore` | undo a soft delete |
| `POST` | `/api/v1/<resource>/batch` | several operations in one request |
| `GET` | `/api/v1/<resource>/export` | export (JSON, NDJSON, CSV) |
| `POST` | `/api/v1/<resource>/import` | import (JSON, NDJSON, CSV) |

Example resources: `users`, `teams`, `messages`, `documents`, `events`, `tasks`, `contacts`, `files`, `tags`, `notifications`, `integrations`.

## Authentication

### API key

```
GET /api/v1/documents HTTP/1.1
Host: hub.example.org
Authorization: Bearer fs_live_9f2c1a…
Accept: application/json
```

### Standard headers

| Header | Description |
|--------|-------------|
| `Authorization` | `Bearer <token>` or `Basic <base64>` |
| `X-Api-Key` | alternative to the bearer token |
| `X-Workspace-Id` | target workspace in multi-tenant setups |
| `X-Request-Id` | UUID for tracing, is returned |
| `Accept-Language` | `de`, `en`, `fr` |
| `If-Match` | ETag for the optimistic lock |
| `Idempotency-Key` | UUID for repeatable `POST` |

### OAuth2

| Step | Endpoint |
|------|----------|
| authorization | `GET /api/v1/oauth/authorize` |
| token | `POST /api/v1/oauth/token` (`authorization_code`, `client_credentials`, `refresh_token`) |
| scopes | `read:documents`, `write:documents`, `read:messages`, `admin` |

## Response envelope

```json
{
  "data": [ { "id": "doc_01HZXD", "title": "Introduction" } ],
  "meta": {
    "total": 132,
    "limit": 25,
    "nextCursor": "eyJvZmZzZXQiOjI1fQ",
    "requestId": "b2c1…"
  },
  "links": {
    "self": "/api/v1/documents?limit=25",
    "next": "/api/v1/documents?cursor=eyJvZmZzZXQiOjI1fQ"
  }
}
```

Single resource:

```json
{
  "data": {
    "id": "doc_01HZXD",
    "title": "Introduction",
    "status": "published",
    "createdAt": "2026-10-01T09:00:00+02:00",
    "updatedAt": "2026-10-28T11:04:00+01:00"
  },
  "meta": { "etag": "W/\"7-a1b2\"" }
}
```

## Pagination

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | number | `25` | 1–100 entries |
| `cursor` | string | - | cursor of the next page |
| `offset` | number | `0` | alternative to the cursor |

```bash
curl -H "Authorization: Bearer $FS_TOKEN" \
  "https://hub.example.org/api/v1/tasks?limit=50&cursor=eyJvZmZzZXQiOjUwfQ"
```

## Filter, sorting, search

| Parameter | Example | Description |
|-----------|---------|-------------|
| `filter` | `filter=status:eq:open` | `field:op:value` with `eq`, `ne`, `gt`, `gte`, `lt`, `lte`, `in`, `like`, `null` |
| `filter` (several) | `filter=status:eq:open;dueAt:lt:2026-12-01` | combined with `;` (AND) |
| `sort` | `sort=-dueAt,title` | `-` = descending |
| `q` | `q=budget` | full-text search |
| `fields` | `fields=id,title,status` | restricting the fields (sparse fieldsets) |
| `expand` | `expand=assignees,tags` | expand references |
| `includeDeleted` | `true` | also deliver soft-deleted items |

```bash
curl -H "Authorization: Bearer $FS_TOKEN" \
  "https://hub.example.org/api/v1/tasks?filter=status:eq:open;assigneeId:eq:user_01HZXA&sort=-dueAt&fields=id,title,dueAt&limit=25"
```

## Creating and changing

```bash
curl -X POST "https://hub.example.org/api/v1/tasks" \
  -H "Authorization: Bearer $FS_TOKEN" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: 6f0f…" \
  -d '{
    "title": "Finish the documentation",
    "boardId": "board_01HZY2",
    "assigneeIds": ["user_01HZXA"],
    "priority": "high",
    "dueAt": "2026-11-15T17:00:00+01:00"
  }'
```

```bash
curl -X PATCH "https://hub.example.org/api/v1/tasks/task_01HZXG" \
  -H "Authorization: Bearer $FS_TOKEN" \
  -H "Content-Type: application/json" \
  -H 'If-Match: W/"3-88ac"' \
  -d '{ "status": "done" }'
```

## Batch

```json
{
  "operations": [
    { "op": "create", "resource": "tasks", "data": { "title": "A" } },
    { "op": "update", "resource": "tasks", "id": "task_01HZXG", "data": { "status": "done" } },
    { "op": "delete", "resource": "tasks", "id": "task_01HZXH" }
  ]
}
```

Answer per operation with `index`, `status` and `data` or `error`; aborts with `stopOnError: true`.

## Error format

```json
{
  "error": {
    "code": "validation_error",
    "message": "required field 'title' missing",
    "status": 400,
    "requestId": "b2c1…",
    "fields": [{ "field": "title", "issue": "required" }]
  }
}
```

| HTTP | code | Meaning |
|------|------|---------|
| 400 | `validation_error` | payload invalid |
| 401 | `unauthorized` | no token or wrong token |
| 403 | `forbidden` | permission missing |
| 404 | `not_found` | resource does not exist |
| 409 | `conflict` | ETag conflict, duplicate or lock |
| 413 | `payload_too_large` | body too large |
| 422 | `unprocessable` | schema valid, domain rule violated |
| 429 | `rate_limited` | quota exhausted |
| 500 | `internal_error` | server error |
| 503 | `unavailable` | maintenance, replication lag |

## Synchronisation

| Mechanism | Description |
|-----------|-------------|
| `ETag` + `If-Match` | optimistic lock on `PATCH`/`PUT` |
| `Last-Modified` + `If-None-Match` | cache, answer `304` |
| `updatedSince` | delta query: `GET /tasks?updatedSince=2026-10-30T00:00:00Z` |
| `deletedSince` | synchronisation that includes deleted records |
| `syncToken` | persistent marker for full/delta comparison |

## Rate limits

| Area | Standard |
|------|----------|
| standard token | 600 requests / minute |
| write operations | 120 / minute |
| batch | 30 requests / minute, max. 100 operations |
| import/export | 10 / hour |

Headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`, `Retry-After`.

## Webhooks

```bash
curl -X POST "https://hub.example.org/api/v1/integrations" \
  -H "Authorization: Bearer $FS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "kind": "webhook",
    "targetUrl": "https://hooks.example.org/fs",
    "events": ["document.created", "task.updated"],
    "secret": "whsec_…"
  }'
```

| Event | Triggered by |
|-------|--------------|
| `document.created` / `.updated` / `.published` / `.deleted` | wiki page |
| `message.created` / `.updated` / `.deleted` | chat message |
| `task.created` / `.updated` / `.moved` / `.deleted` | task |
| `event.created` / `.updated` / `.cancelled` | appointment |
| `contact.created` / `.updated` | contact |
| `file.uploaded` / `.deleted` | file |
| `user.invited` / `.disabled` | account |

Payload:

```json
{
  "id": "evt_01HZY9",
  "type": "task.updated",
  "createdAt": "2026-10-31T06:20:00+01:00",
  "data": {
    "id": "task_01HZXG",
    "changes": [{ "field": "status", "from": "open", "to": "done" }]
  }
}
```

Signature header: `X-FS-Signature: sha256=<hmac(secret, body)>`; delivery attempts with backoff (1 min, 5 min, 30 min, 2 h, 6 h).

## SDKs and clients

| Language | Package | Example |
|----------|---------|---------|
| JavaScript/TypeScript | `@freesynergy/client` | `client.tasks.list({ filter: "status:eq:open" })` |
| Python | `freesynergy` | `client.tasks.list(status="open")` |
| PHP | `freesynergy/sdk` | `$client->tasks->list(['status' => 'open'])` |
| CLI | `fs` | `fs tasks list --status open --limit 25` |

### Python example

```python
from freesynergy import Client

client = Client(token="fs_live_9f2c1a")

for task in client.tasks.list(filter="status:eq:open", sort="-dueAt"):
    print(task.id, task.title, task.due_at)

client.tasks.update(
    task.id,
    data={"status": "done"},
    if_match=task.etag,
)
```

## See also

- [standard models](models.md)
- [standard fields](fields.md)
- [data formats](formats.md)
- [conventions](conventions.md)
- [desktop frontends — API examples](../programs/platform/desktop/frontends/)
- [workflows](../programs/suites/workflows/)
