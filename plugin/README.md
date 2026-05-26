# rs Bridge Plugin

This plugin registers the current Roblox Studio session with the local `rs` bridge, long-polls for queued commands, dispatches the current Studio handlers, and posts structured results back to the bridge.

Current plugin protocol: `5`. Current plugin version: `0.4.0`.

Build with Rojo:

```powershell
rojo build default.project.json --output rs-bridge-plugin.rbxmx
```

Install the generated `.rbxmx` as a local Studio plugin and restart Studio.

The plugin expects the bridge at `http://127.0.0.1:7878` by default. Change `src/Config.lua` only for local development when the bridge is intentionally running on another port.

## Local Bridge Shape

The bridge is local-only and listens on `127.0.0.1`. The plugin uses public plugin routes:

- `POST /register` to announce `{id, name, placeFilePath, protocolVersion, pluginVersion, capabilities}` and receive a Studio session token.
- `GET /poll/{sessionToken}` to long-poll for one queued command.
- `POST /heartbeat/{sessionToken}` to keep the registered Studio session fresh.
- `POST /result/{commandId}` to return `{ok, data?, error?}` for a dispatched command.

CLI-facing bridge routes use the local `x-rs-bridge-token` header that the `rs` binary attaches. The plugin does not need `RS_BRIDGE_TOKEN`; it uses the session token returned from `/register`.

## Registered Capabilities

The plugin registers this exact capability set in `src/Main.server.lua`:

- `exec` -> `Handlers/Exec.lua`
- `read` -> `Handlers/Read.lua`
- `export` -> `Handlers/Export.lua`
- `importAsset` -> `Handlers/ImportAsset.lua`
- `importImage` -> `Handlers/ImportImage.lua`
- `importUploaded` -> `Handlers/ImportUploaded.lua`
- `importUiPack` -> `Handlers/ImportUiPack.lua`
- `importAudio` -> `Handlers/ImportAudio.lua`
- `validate` -> `Handlers/Validate.lua`
- `repairTool` -> `Handlers/RepairTool.lua`
- `snapshot` -> `Handlers/Snapshot.lua`
- `create` -> `Handlers/Create.lua`
- `upsertFiles` -> `Handlers/UpsertFiles.lua`
- `applyPlan` -> `Handlers/ApplyPlan.lua`
- `packageUpdate` -> `Handlers/PackageUpdate.lua`
- `history` -> `Handlers/History.lua`
- `deps` -> `Handlers/Deps.lua`
- `serialize` -> `Handlers/Serialize.lua`
- `deserialize` -> `Handlers/Deserialize.lua`
- `autopilotReview` -> `AutopilotReview.handle`

Keep this list synchronized with `src/Main.server.lua`; CI runs a static drift test for it.
