# Protocol

All bridge endpoints use JSON.

## Plugin Endpoints

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/register` | Register a Studio session with `{id, name, placeFilePath, protocolVersion, pluginVersion, capabilities}` |
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
| `POST` | `/import-uploaded` | Create Studio image/audio instances from existing Roblox asset IDs |
| `POST` | `/import-ui-pack` | Build multiple decoded PNG payloads into one Studio UI container |
| `POST` | `/import-audio` | Create Sound instances from Roblox audio asset IDs |
| `POST` | `/validate` | Report broken refs, Tool wiring, asset, and path diagnostics |
| `POST` | `/repair-tool` | Repair Tool welds and equip-ready physics settings |
| `POST` | `/snapshot` | Summarize a subtree inventory without returning the full tree |
| `POST` | `/create` | Create one Studio Instance with typed properties |
| `POST` | `/upsert-files` | Upsert local script/config file payloads into Studio |
| `POST` | `/apply-plan` | Apply approved safe operations from `diff --fix-plan` |
| `POST` | `/package-update` | Reapply a package using `rsSourceId`/`rsPackageId` ownership metadata |
| `POST` | `/history` | List/show/undo Studio-side command audit records |
| `POST` | `/deps` | Report asset, script, remote, and ownership dependencies |
| `POST` | `/serialize` | Serialize a subtree into the transfer blob format |
| `POST` | `/deserialize` | Deserialize a transfer blob under a Studio parent |
| `POST` | `/autopilot-review` | Publish or read the latest Autopilot run summary for the Studio review panel |
| `POST` | `/transfer` | Serialize from source and deserialize into target |
| `POST` | `/shutdown` | Graceful bridge shutdown |

Responses use this envelope except `/healthz`:

```json
{ "ok": true, "data": {} }
```

## Versioning

The plugin registration payload includes:

```json
{
  "id": "studio-guid",
  "name": "Snipe a Slime!",
  "placeFilePath": "D:\\Snipe a Slime!\\place.rbxl",
  "protocolVersion": 5,
  "pluginVersion": "0.4.0",
  "capabilities": ["exec", "read", "validate", "deserialize", "applyPlan", "packageUpdate", "deps", "autopilotReview"]
}
```

The bridge rejects Studio-backed CLI commands when the registered plugin protocol does not match
the CLI's expected `PLUGIN_PROTOCOL_VERSION`. `rs doctor` reports unknown or stale protocol
registrations as old loaded plugin sessions and points the user at the plugin reinstall/restart fix.

`/deserialize` accepts optional package-import conflict controls:

```json
{
  "studio": "Snipe a Slime!",
  "parentPath": "ServerStorage",
  "blob": {},
  "conflictMode": "fail",
  "dryRun": false,
  "rollbackOnError": true,
  "packageId": "rspkg-..."
}
```

`conflictMode` is one of `fail`, `replace`, `merge`, or `rename`. `dryRun` returns the planned root
path and conflict status without mutating Studio. `rollbackOnError` lets the plugin restore replaced
content when replacement was backed up before deserialize, and `packageId` is stamped on created
instances as ownership metadata.

`/autopilot-review` accepts `{action:"set"|"get"|"clear", run?}`. `set` stores the latest
`rs.autopilot.review.v1` summary in the plugin and refreshes the Studio dock widget. The payload is
review metadata only: run ID, status, prompt, risk, operation counts, changed paths, warnings,
rollback state, and artifact paths. It does not approve or apply mutations.

`/apply-plan` accepts a JSON fix plan produced by `rs diff --fix-plan --format json`:

```json
{
  "studio": "Snipe a Slime!",
  "rootPath": "Workspace.Tool",
  "plan": { "safeToApply": true, "changes": [] },
  "dryRun": true,
  "approved": false,
  "only": ["added", "modified"],
  "exclude": ["Scripts"],
  "force": false
}
```

The plugin only mutates when `approved` is true, refuses unsafe plans with conflicts, and uses
ownership attributes to prevent overwriting manual instances unless `force` is true.

`/package-update` accepts `{parentPath, blob, packageId, mode, dryRun, force}` where `mode` is
`owned-only`, `preserve-local`, `replace-owned`, or `conflict-report`. Imported/package instances
are stamped with `rsSourceId`, `rsPackageId`, `rsImportedAt`, and `rsManagedBy`.

`/history` accepts `{action:"list"|"show"|"undo", commandId?}`. Mutating handlers may include
rollback snapshots in their result data; `undo` restores those snapshots when present.

`/transfer` forwards the same deserialize controls after source serialization:

```json
{
  "fromStudio": "Source",
  "fromPath": "ServerStorage.Tool",
  "toStudio": "Target",
  "toParentPath": "ServerStorage",
  "conflictMode": "replace",
  "dryRun": false,
  "rollbackOnError": true
}
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
