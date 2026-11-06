# Mastodon - API documentation

## Overview
Mastodon offers a REST API for access to all social features. The API allows reading, writing and managing toots, users, accounts and media.

## Authentication
Mastodon uses OAuth2:

- **OAuth2 authorization code flow**: for web apps
- **OAuth2 client credentials**: for app-only access

### Headers
```
Authorization: Bearer <access-token>
Content-Type: application/json
```

## Endpoints

### Accounts
| Method | Endpoint | Description |
|---------|----------|--------------|
| GET | `/api/v1/accounts/:id` | Account details |
| GET | `/api/v1/accounts/verify_credentials` | Current account |
| PATCH | `/api/v1/accounts/update_credentials` | Update the account |
| GET | `/api/v1/accounts/:id/statuses` | Status updates |
| POST | `/api/v1/accounts/:id/follow` | Follow the account |
| POST | `/api/v1/accounts/:id/unfollow` | Unfollow the account |

### Toots (statuses)
| Method | Endpoint | Description |
|---------|----------|--------------|
| GET | `/api/v1/timelines/home` | Home timeline |
| GET | `/api/v1/timelines/public` | Public timeline |
| GET | `/api/v1/timelines/tag/:hashtag` | Hashtag timeline |
| GET | `/api/v1/statuses/:id` | Status details |
| POST | `/api/v1/statuses` | Create a status |
| DELETE | `/api/v1/statuses/:id` | Delete a status |
| POST | `/api/v1/statuses/:id/favourite` | Favourite a status |
| POST | `/api/v1/statuses/:id/reblog` | Reblog |
| POST | `/api/v1/statuses/:id/mute` | Mute a status |
| POST | `/api/v1/statuses/:id/bookmark` | Bookmark a status |

### Media
| Method | Endpoint | Description |
|---------|----------|--------------|
| POST | `/api/v1/media` | Upload media |
| GET | `/api/v1/media/:id` | Media details |

### Notifications
| Method | Endpoint | Description |
|---------|----------|--------------|
| GET | `/api/v1/notifications` | Notifications |
| POST | `/api/v1/notifications/:id/dismiss` | Dismiss a notification |
| DELETE | `/api/v1/notifications` | Dismiss all |

### Followers
| Method | Endpoint | Description |
|---------|----------|--------------|
| GET | `/api/v1/accounts/:id/followers` | Followers |
| GET | `/api/v1/accounts/:id/following` | Following |

### Search
| Method | Endpoint | Description |
|---------|----------|--------------|
| GET | `/api/v1/search` | Search (accounts, hashtags, statuses) |

### Mutes
| Method | Endpoint | Description |
|---------|----------|--------------|
| GET | `/api/v1/mutes` | Muted accounts |
| POST | `/api/v1/accounts/:id/mute` | Mute an account |
| POST | `/api/v1/accounts/:id/unmute` | Unmute an account |

### Blocks
| Method | Endpoint | Description |
|---------|----------|--------------|
| GET | `/api/v1/blocks` | Blocked accounts |
| POST | `/api/v1/accounts/:id/block` | Block an account |
| POST | `/api/v1/accounts/:id/unblock` | Unblock an account |

### Filters
| Method | Endpoint | Description |
|---------|----------|--------------|
| GET | `/api/v1/filters` | Filters |
| POST | `/api/v1/filters` | Create a filter |
| PUT | `/api/v1/filters/:id` | Update a filter |

### Instances
| Method | Endpoint | Description |
|---------|----------|--------------|
| GET | `/api/v1/instance` | Instance info |
| GET | `/api/v1/instance/peers` | Federated instances |

## Examples

### Creating a status
```bash
curl -X POST "https://mastodon.example.com/api/v1/statuses" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "status": "Hello from the Mastodon API!",
    "visibility": "public"
  }'
```

### A status with media
```bash
# 1. upload the media
curl -X POST "https://mastodon.example.com/api/v1/media" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -F "file=@image.jpg"

# 2. create the status with the media id
curl -X POST "https://mastodon.example.com/api/v1/statuses" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "status": "Sharing an image",
    "media_ids": ["MEDIA_ID_HERE"]
  }'
```

### Python example
```python
import requests
import json

MASTODON_API = "https://mastodon.example.com/api/v1"
ACCESS_TOKEN = "your-access-token"

headers = {
    "Authorization": f"Bearer {ACCESS_TOKEN}",
    "Content-Type": "application/json"
}

# create a status
data = {
    "status": "Hello, this is a test toot!",
    "visibility": "public"
}
response = requests.post(
    f"{MASTODON_API}/statuses",
    json=data,
    headers=headers
)
print(response.json())

# home timeline
response = requests.get(
    f"{MASTODON_API}/timelines/home",
    headers=headers
)
print(response.json())

# update the account
profile_data = {
    "display_name": "My name",
    "note": "My description"
}
response = requests.patch(
    f"{MASTODON_API}/accounts/update_credentials",
    json=profile_data,
    headers=headers
)
print(response.json())
```

### Node.js example
```javascript
const axios = require('axios');

const MASTODON_API = 'https://mastodon.example.com/api/v1';
const ACCESS_TOKEN = 'your-access-token';

const headers = {
    'Authorization': `Bearer ${ACCESS_TOKEN}`,
    'Content-Type': 'application/json'
};

// create a status
const data = {
    status: 'Hello from Node.js!',
    visibility: 'public'
};

axios.post(`${MASTODON_API}/statuses`, data, { headers })
    .then(response => console.log(response.data))
    .catch(error => console.error(error));

// home timeline
axios.get(`${MASTODON_API}/timelines/home`, { headers })
    .then(response => console.log(response.data))
    .catch(error => console.error(error));
```

### Media upload (Python)
```python
import requests

MASTODON_API = "https://mastodon.example.com/api/v1"
ACCESS_TOKEN = "your-access-token"

headers = {
    "Authorization": f"Bearer {ACCESS_TOKEN}"
}

# upload the media
with open('image.jpg', 'rb') as f:
    files = {'file': f}
    response = requests.post(
        f"{MASTODON_API}/media",
        headers=headers,
        files=files
    )
    media_id = response.json()['id']
    print(f"Media ID: {media_id}")
```

## Rate limits
- Standard: 300 requests per 5 minutes per token
- Response headers:
  - `X-RateLimit-Limit`: maximum requests (300)
  - `X-RateLimit-Remaining`: requests left
  - `X-RateLimit-Reset`: reset time (Unix timestamp)

## Webhooks
Mastodon has no built-in webhook system, but:
- **Streaming API**: real time over server-sent events
- **Polling**: regular status queries

### Streaming API
```bash
curl -X GET "https://mastodon.example.com/api/v1/streaming/?stream=user" \
  -H "Authorization: Bearer YOUR_ACCESS_TOKEN"
```

Stream types:
- `user`: user notifications
- `public`: public timeline
- `hashtag`: hashtag stream
- `direct`: direct messages

## SDKs and client libraries
- **JavaScript**: `toot` (officially recommended)
- **Python**: `python-mastodon`
- **Ruby**: `mastodon-api`
- **Go**: `go-mastodon`
- **Java**: `mastodon4j`

Installing:
```bash
# npm
npm install toot

# pip
pip install Mastodon.py

# Go
go get github.com/mattn/go-mastodon
```

## See also
- [menus.md](menus.md)
- [features.md](features.md)
- [description.md](description.md)
- [layout.md](layout.md)
- [shortcuts.md](shortcuts.md)
- [api.md](api.md) (this file)
