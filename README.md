# rs - Roblox Studio CLI

A single-binary CLI plus Luau Studio plugin for programmatic Roblox Studio control.

## What It Does

| Command | Purpose |
|---|---|
| `rs list` | Show connected Studios |
| `rs exec --studio X --lua "code"` | Run Luau in a Studio and return JSON |
| `rs read --studio X --path Workspace --depth 2` | Read a rich JSON instance tree |
| `rs export --studio X --path ServerStorage.Foo --out ./export` | Save a subtree as individual local files |
| `rs transfer --from "A:Path" --to "B:ParentPath"` | Copy an instance tree from Studio A to Studio B |
| `rs bridge serve/status/stop` | Manage the local bridge daemon |

The bridge listens on `127.0.0.1:7878` by default and can be changed with `--port` or `RS_BRIDGE_PORT`.

## Build

```powershell
cargo build --release
```

The binary will be at `target\release\rs.exe`.

## Install The Studio Plugin

```powershell
cd plugin
rojo build default.project.json --output rs-bridge-plugin.rbxmx
```

Drop `rs-bridge-plugin.rbxmx` into your local Roblox Studio plugins folder:

- Windows: `%LOCALAPPDATA%\Roblox\Plugins\`
- macOS: `~/Documents/Roblox/Plugins/`

Restart Studio. The plugin registers with the local bridge and starts polling for commands.

## Quick Start

```powershell
rs list
rs exec --studio "Snipe a Slime!" --lua "return #game.ReplicatedStorage:GetChildren()"
rs read --studio "Snipe a Slime!" --path "Workspace" --depth 1
rs export --studio "Snipe a Slime!" --path "ServerStorage.SniperSkins" --out ".\exports\snipers"
rs transfer --from "Snipe for Brainrots!:ServerStorage.SniperSkins" --to "Snipe a Slime!:ServerStorage"
```

The bridge auto-spawns on first CLI command and stays running until `rs bridge stop`.

`export` writes one `instance.json` metadata file per Studio instance. Scripts are emitted as
`.server.lua`, `.client.lua`, or `.module.lua`. Roblox-hosted meshes, textures, images,
audio, animations, and VFX textures are emitted as individual `.asset.json` reference files
containing the source property and asset URI.

## Docs

- [Architecture](docs/architecture.md)
- [Protocol](docs/protocol.md)
- [Transfer format](docs/transfer-format.md)

## License

MIT
