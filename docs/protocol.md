# Protocol

All bridge endpoints use JSON.

## Plugin Endpoints

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/register` | Register a Studio session with `{id, name, placeFilePath}` |
| `GET` | `/poll/{sessionToken}` | Long-poll for one pending command |
| `POST` | `/result/{commandId}` | Submit `{ok, data?, error?}` |
| `POST` | `/heartbeat/{sessionToken}` | Keep the session alive |

## CLI Endpoints

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/healthz` | Liveness |
| `GET` | `/studios` | Connected Studio list |
| `POST` | `/exec` | Enqueue Luau execution |
| `POST` | `/read` | Enqueue instance-tree read |
| `POST` | `/export` | Extract a subtree into individual file payloads |
| `POST` | `/import-asset` | Build local mesh payloads into welded Studio MeshParts |
| `POST` | `/import-image` | Build local PNG pixel payloads into Studio GUI image objects |
| `POST` | `/transfer` | Serialize from source and deserialize into target |
| `POST` | `/shutdown` | Graceful bridge shutdown |

Responses use this envelope except `/healthz`:

```json
{ "ok": true, "data": {} }
```

```json
{ "ok": false, "error": "human-readable", "code": "machine-readable" }
```

Plugin poll responses contain one command or `null`:

```json
{
  "ok": true,
  "data": {
    "commandId": "uuid",
    "type": "exec",
    "payload": { "lua": "return 1 + 1" }
  }
}
```
