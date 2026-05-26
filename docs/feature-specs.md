# Feature Specs

This document captures proposed next features for `rs`, the Roblox Studio CLI plus Studio plugin. Each feature is scoped as a production implementation target: user value, command shape, bridge/plugin responsibilities, validation, and notable risks.

Implementation status: initial v1 command, bridge, and plugin support has been added for every feature below. A second implementation pass added the next reliability layer: `doctor`, plugin/CLI protocol versioning, `validate --fix`, live smoke commands, better asset import fidelity, Open Cloud upload, and package import conflict modes. A third pass added `install-plugin`, upload waiting/import composition, Open Cloud profiles, package verification and archives, diff fix plans, transactional transfer controls, smoke regression reports, asset fidelity v2, and ownership metadata. Some items deliberately start with conservative behavior where Roblox platform limits make a fully automatic path unsafe, especially local audio import, raw mesh cloud upload, credential storage, and destructive sync/package semantics.

Before adding new command families, use [Current Feature Expansion Plan](current-feature-expansion-plan.md) to deepen the existing features across object coverage, property coverage, references, asset ownership, safety controls, idempotency, and proof artifacts.

## Latest Implemented Features

| Feature | Status |
|---|---|
| `rs doctor` | Checks bridge health, connected Studios, installed plugin bundle path/timestamp, loaded plugin protocol/version, and prints exact restart/install fixes. `--fix` starts the bridge and copies the repo-built plugin bundle when safe. |
| Plugin/CLI protocol versioning | Plugin registration includes `protocolVersion`, `pluginVersion`, and `capabilities`; Studio commands reject stale plugin sessions before dispatch. |
| `rs validate --fix` | Runs validation, applies safe `repair-tool` fixes when diagnostics opt into that fix, then reruns validation and reports before/after. |
| `rs smoke validate/import-ui-pack/repair-tool/all` | Creates small live Studio fixtures, exercises the relevant command path, verifies state, and cleans up. |
| Better `import-asset` fidelity | Preserves OBJ/glTF material names, base colors, texture URI metadata, object hierarchy paths, and source pivots as direct properties or `rs*` attributes. |
| `rs upload` | Uses Roblox Open Cloud asset upload for image, audio, and supported model-container files such as FBX/glTF/GLB; raw OBJ/STL still use `import-asset`. |
| Package conflict modes | `package import` supports `--if-exists fail|replace|merge|rename` plus `--dry-run`. |
| `rs install-plugin` | Builds with Rojo, installs the plugin bundle, verifies hash/timestamp, and lists Studio windows that need restart. |
| `rs upload --wait` / `--import-to` | Polls Open Cloud operations until a final asset ID is available, then can import image/audio assets into Studio. |
| `rs import-uploaded` | Creates Studio image/audio instances from existing Roblox asset IDs. |
| Open Cloud profiles | `rs auth profile add/list/remove/default` stores local creator/API-key profiles for upload commands without printing secrets. |
| `rs package verify/pack/unpack` | Verifies manifest/checksums/blob/asset refs/conflicts and moves package folders through zip archives. |
| `rs diff --fix-plan` | Converts diff output into a conservative, JSON-friendly mutation plan. |
| Transactional transfer/import controls | `transfer --dry-run --replace --rollback-on-error` and package import rollback controls route through deserialize safety checks. |
| `rs smoke regression` | Writes JSON reports for broader weld transfer, conflict dry-run, Tool equip, and upload-mock coverage. |
| Stable ownership metadata | Imported, synced, and package-created instances receive `rsSourceId`, `rsPackageId`, `rsImportedAt`, and `rsManagedBy` attributes. |

## Priorities

| Priority | Feature | Why |
|---|---|---|
| P0 | `validate` | Catches the broken-reference, Tool, weld, and asset issues that are hardest to debug after import or transfer. |
| P0 | `repair-tool` / `wire-tool` | Turns the most common imported Tool failure into a one-command fix. |
| P1 | `diff` | Makes Studio changes reviewable before overwriting or transferring content. |
| P1 | `sync-folder` | Enables an edit-save-push loop for scripts, UI images, and exported assets. |
| P1 | `import-ui-pack` | Extends the existing PNG importer into real UI assembly workflows. |
| P2 | `snapshot` | Gives quick project inventory and diagnostics without writing files. |
| P2 | `create` | Adds safe instance creation for automation scripts and setup tasks. |
| P2 | `batch` | Runs common import/export/transfer jobs from one manifest. |
| P3 | `package` | Creates a portable interchange format for review, archiving, and reimport. |
| P3 | `import-audio` | Useful, but Roblox audio upload and permission constraints need careful design. |

## 1. `rs validate`

### Goal

Inspect a Studio subtree and report production issues that are invisible or painful to diagnose from the Explorer alone.

### Command

```powershell
rs validate --studio "Snipe a Slime!" --path "Workspace.MyTool"
rs validate --studio "Snipe a Slime!" --path "ServerStorage" --format json
rs validate --studio "Snipe a Slime!" --path "Workspace" --rules tool,welds,refs,assets
```

### Checks

- Broken object-reference properties:
  - `Weld.Part0`, `Weld.Part1`
  - `Motor6D.Part0`, `Motor6D.Part1`
  - `WeldConstraint.Part0`, `WeldConstraint.Part1`
  - `ObjectValue.Value`
  - `Attachment` references such as constraints using `Attachment0` / `Attachment1`
  - UI references such as `Adornee`
- Tool readiness:
  - Missing `Handle`
  - Multiple ambiguous handles
  - Anchored `BasePart` descendants
  - Parts not rigidly connected to `Handle`
  - Handle collision/mass settings that are likely to break equip behavior
- Asset readiness:
  - Empty or invalid `MeshPart.MeshId`
  - Empty `ImageLabel.Image` / `ImageContent` where image content is expected
  - Missing `Sound.SoundId`
  - Roblox asset URI properties that failed to serialize or import
- Transfer/import health:
  - Instances with duplicated sibling names when paths would be ambiguous
  - Unsupported or skipped classes under the selected subtree
  - Scripts with empty source when source was expected
  - Excessively large imported meshes or editable images

### CLI Output

Default output is human-readable and grouped by severity:

```text
FAIL  Workspace.MyTool.Handle.Weld_12 Part0 is nil
WARN  Workspace.MyTool.Blade is anchored inside Tool
INFO  Workspace.MyTool contains 44 rigid joints

2 fail, 3 warn, 8 info
```

`--format json` returns structured diagnostics:

```json
{
  "summary": { "fail": 2, "warn": 3, "info": 8 },
  "diagnostics": [
    {
      "severity": "fail",
      "rule": "weld.ref.missing",
      "path": "Workspace.MyTool.Handle.Weld_12",
      "property": "Part0",
      "message": "Part0 is nil"
    }
  ]
}
```

### Bridge And Plugin Work

- Add `ValidateRequest` and `ValidateResponse` protocol structs.
- Add `POST /validate` bridge endpoint.
- Add plugin command handler `validate`.
- Implement rule modules in Luau, with each diagnostic carrying:
  - severity
  - rule id
  - instance path
  - optional property name
  - message
  - optional fix id for repairable issues
- Use Roblox instance/type checks rather than class-name string guessing where possible.

### Validation

- Unit-test request/response JSON round trips.
- Plugin smoke test with:
  - valid welded Tool
  - Tool with missing `Handle`
  - Tool with anchored part
  - Weld with nil `Part0` / `Part1`
  - ObjectValue pointing outside the validated subtree
- Live Studio test should confirm both text and JSON output.

### Risks

- Some reference properties may not be writable or readable in plugin security context.
- Connectivity analysis must follow rigid joints carefully enough to avoid false positives.
- Asset validity can only be partially verified without cloud asset fetches.

## 2. `rs repair-tool` / `rs wire-tool`

### Goal

Repair imported or transferred Tools so they equip reliably. This should handle the common case where MeshParts import correctly but rigid body connections are missing or broken.

### Command

```powershell
rs repair-tool --studio "Snipe a Slime!" --path "Workspace.Rifle"
rs wire-tool --studio "Snipe a Slime!" --path "Workspace.Rifle" --handle Handle
rs wire-tool --studio "Snipe a Slime!" --path "Workspace.Rifle" --dry-run
```

`repair-tool` and `wire-tool` can be aliases for the same implementation. `repair-tool` reads better for end users; `wire-tool` reads better for asset-pipeline users.

### Behavior

- Locate the target `Tool`.
- Locate the handle:
  - Prefer child named `Handle`.
  - Allow `--handle <name>` override.
  - Fail if no clear handle exists.
- Collect all `BasePart` descendants under the Tool.
- For each non-handle part:
  - Remove or report broken joints that connect to nil parts.
  - Create a `WeldConstraint` from `Handle` to the part if no rigid connection exists.
  - Preserve existing valid `Motor6D`, `Weld`, or `WeldConstraint` connections by default.
- Set Tool-ready physics defaults unless disabled:
  - `Anchored = false`
  - usually `CanCollide = false` for non-handle parts
  - optionally `Massless = true` for decorative parts
- Return a clear repair report.

### Options

```text
--dry-run              Report intended changes without mutating Studio
--handle <name>        Use a specific child as the handle
--replace-broken       Delete broken joints instead of leaving them in place
--no-physics-fix       Do not modify Anchored, CanCollide, or Massless
--collision on|off     Explicit collision setting for non-handle parts
--massless on|off      Explicit massless setting for non-handle parts
```

### Bridge And Plugin Work

- Add `RepairToolRequest` and `RepairToolResponse`.
- Add `POST /repair-tool` bridge endpoint.
- Add plugin handler that performs all mutations inside Studio.
- Reuse `validate` connectivity logic so repair and validation agree.
- Return before/after counts:
  - parts found
  - valid joints preserved
  - broken joints found
  - welds created
  - physics properties changed

### Validation

- Fixture with Tool containing two loose MeshParts: creates one weld.
- Fixture with valid Motor6D rig: preserves Motor6D.
- Fixture with nil `Weld.Part0`: reports or removes depending on `--replace-broken`.
- Live Studio smoke: call `Humanoid:EquipTool(transferredTool)` and verify the Tool moves to the character after a short wait.

### Risks

- Some Tools intentionally use articulated `Motor6D` setups; avoid flattening valid rigs.
- Blindly changing collision can alter gameplay. Keep options explicit and report every changed property.

## 3. `rs diff`

### Goal

Compare Studio state against another Studio session, an exported folder, or a transfer blob before applying changes.

### Command

```powershell
rs diff --studio "Place A" --path "ServerStorage.SniperSkins" --against-studio "Place B" --against-path "ServerStorage.SniperSkins"
rs diff --studio "Place A" --path "Workspace.Map" --against-export ".\exports\map"
rs diff --export ".\exports\old" --against-export ".\exports\new" --format json
```

### Behavior

- Serialize both sides into the same normalized comparison shape.
- Compare:
  - instance existence
  - class names
  - parent/child layout
  - selected properties
  - attributes
  - tags
  - script source hashes
  - asset URI references
  - object-reference topology inside the subtree
- Ignore volatile properties by default:
  - debug IDs
  - timestamps
  - generated import IDs
  - transient Studio-only runtime state

### Output

```text
M ServerStorage.SniperSkins.Rifle.Handle.MeshId
A ServerStorage.SniperSkins.Rifle.Icon
D ServerStorage.SniperSkins.LegacyRifle
R ServerStorage.SniperSkins.Rifle.Handle.Weld.Part1
```

JSON output should include path, change type, before value, after value, and property name when relevant.

### Bridge And Plugin Work

- Reuse serializer/export reader where possible.
- Add local export loader in Rust.
- Add comparison module in Rust rather than Luau so export-vs-export does not require Studio.
- Add options:
  - `--include <prop>`
  - `--exclude <prop>`
  - `--ignore-scripts`
  - `--ignore-assets`

### Validation

- Unit tests for added, deleted, modified, and reference-topology changes.
- Export-vs-export tests do not need Studio.
- Studio-vs-Studio live smoke with a small modified fixture.

### Risks

- Path-based matching breaks on sibling duplicate names. The diff engine should prefer stable synthetic IDs when available and report ambiguous path matching.

## 4. `rs sync-folder`

### Goal

Create a practical development loop where local source files and assets can be pushed into an open Studio place repeatedly.

### Command

```powershell
rs sync-folder --studio "Snipe a Slime!" --folder ".\src" --to "ServerScriptService"
rs sync-folder --studio "Snipe a Slime!" --folder ".\ui" --to "StarterGui" --watch
rs sync-folder --studio "Snipe a Slime!" --folder ".\assets" --to "ReplicatedStorage.Assets" --manifest rs.sync.json
```

### Manifest

```json
{
  "targets": [
    {
      "folder": "src/server",
      "to": "ServerScriptService",
      "patterns": ["*.server.lua"]
    },
    {
      "folder": "ui/icons",
      "to": "StarterGui",
      "patterns": ["*.png"],
      "kind": "icon"
    }
  ]
}
```

### Behavior

- Map local files to Studio instances:
  - `.server.lua` -> `Script`
  - `.client.lua` -> `LocalScript`
  - `.module.lua` -> `ModuleScript`
  - `.lua` -> `ModuleScript` by default unless configured
  - `.png` -> `ImageLabel` / `ImageButton` through existing import-image flow
  - `.obj`, `.stl`, `.gltf`, `.glb`, `.fbx`, etc. -> mesh import flow
  - `.json` -> attributes/config folder when explicitly mapped
- Support `--watch` with debounce.
- Be idempotent:
  - update existing instances by name and type
  - create missing instances
  - avoid deleting by default
- Add `--delete` for explicit mirror mode.

### Bridge And Plugin Work

- Most file parsing stays in Rust.
- Add a plugin-side upsert command that can create/update scripts and folders.
- Reuse import-image and import-asset handlers for asset payloads.
- Add a local manifest parser and change detector in Rust.

### Validation

- Sync scripts into empty Studio container.
- Modify local script and verify Studio source updates.
- Sync PNG into existing `ScreenGui`.
- `--watch` smoke with one file change and one debounce cycle.
- Confirm `--delete` removes only instances owned by sync metadata.

### Risks

- Watch mode can cause accidental churn. Add clear ownership metadata and a dry-run mode before destructive sync.
- Script source write permissions differ by script class and Studio/plugin security context.

## 5. `rs import-ui-pack`

### Goal

Import a folder of PNG UI assets and assemble a usable `ScreenGui` from a small manifest.

### Command

```powershell
rs import-ui-pack --studio "Snipe a Slime!" --folder ".\ui\shop" --to "StarterGui"
rs import-ui-pack --studio "Snipe a Slime!" --manifest ".\ui\shop\ui-pack.json"
```

### Manifest

```json
{
  "name": "ShopGui",
  "to": "StarterGui",
  "elements": [
    {
      "file": "background.png",
      "name": "Background",
      "kind": "image",
      "size": "640x360",
      "position": "0.5,0.5",
      "anchor": "0.5,0.5"
    },
    {
      "file": "buy.png",
      "name": "BuyButton",
      "kind": "button",
      "size": "160x48",
      "position": "0.5,0.8",
      "anchor": "0.5,0.5"
    }
  ]
}
```

### Behavior

- Create or update a `ScreenGui`.
- Import each PNG through the existing editable-image path.
- Create `ImageLabel` or `ImageButton` instances.
- Support layout fields:
  - size in pixels or scale
  - position in pixels or scale
  - anchor point
  - z-index
  - scale type
  - background transparency
- Allow a folder-only mode:
  - no manifest creates a grid of icons/buttons using file names
  - `--kind icon|button|image` applies one default kind to all PNGs

### Bridge And Plugin Work

- Add `ImportUiPackRequest` to carry a list of already-decoded images plus layout metadata.
- Reuse PNG decoding and downscale logic in Rust.
- Add plugin handler that creates the `ScreenGui` once and inserts all elements under it.

### Validation

- Manifest with two PNGs creates one `ScreenGui` and two image objects.
- Re-import updates existing objects without duplicating names.
- Oversized PNG downscales and reports final size.
- Button kind creates `ImageButton` with click-ready defaults.

### Risks

- Editable images are Studio-local content. A future cloud-upload mode may be needed for published games.
- Manifest coordinate parsing must be strict to avoid surprising layouts.

## 6. `rs snapshot`

### Goal

Generate a quick inventory of a subtree for review, debugging, or audit.

### Command

```powershell
rs snapshot --studio "Snipe a Slime!" --path "Workspace"
rs snapshot --studio "Snipe a Slime!" --path "ReplicatedStorage" --format json
rs snapshot --studio "Snipe a Slime!" --path "Workspace.Map" --out ".\snapshot.json"
```

### Behavior

Report:

- total instance count
- class counts
- script counts by type
- asset references by property and URI
- Tool counts and readiness summary
- MeshPart counts
- UI counts
- remote events/functions
- top largest subtrees
- maximum tree depth
- duplicate sibling names

### Bridge And Plugin Work

- Add `snapshot` plugin command that walks the target subtree and aggregates summary data.
- Keep result compact so large places do not return full serialized trees unless requested.
- Add optional `--include-paths` for detailed rows.

### Validation

- Fixture with known class counts.
- Fixture with assets and scripts.
- JSON output round-trip test.

### Risks

- Some properties may be inaccessible; report skipped property names instead of silently omitting them.

## 7. `rs create`

### Goal

Create instances from the CLI safely without writing ad hoc Luau snippets for common setup work.

### Command

```powershell
rs create --studio "Snipe a Slime!" --class Folder --to ReplicatedStorage --name SharedAssets
rs create --studio "Snipe a Slime!" --class Part --to Workspace --name SpawnPad --property Anchored=true --property Size=8,1,8
rs create --studio "Snipe a Slime!" --class RemoteEvent --to ReplicatedStorage.Remotes --name EquipSkin
```

### Behavior

- Validate class creation with `pcall(Instance.new, className)`.
- Set name and parent.
- Set primitive properties from typed CLI values:
  - boolean
  - number
  - string
  - `Vector3`
  - `Color3`
  - `UDim2`
  - enums
- Return created path and class.
- Fail on unknown classes or invalid properties.

### Bridge And Plugin Work

- Add `CreateInstanceRequest`.
- Add typed property decoder shared with deserializer where possible.
- Add `--json` mode for complex properties:

```powershell
rs create --studio X --json create.json
```

### Validation

- Create Folder, Part, RemoteEvent, and ScreenGui.
- Invalid class fails with clear error.
- Invalid property type fails without creating partial hidden state where feasible.

### Risks

- CLI parsing for Roblox types can sprawl quickly. Start with a small, explicit type set.

## 8. `rs batch`

### Goal

Run multiple CLI/plugin operations from a manifest so asset imports, UI imports, exports, validation, and transfers can be repeated consistently.

### Command

```powershell
rs batch --file rs.batch.json
rs batch --file rs.batch.json --dry-run
rs batch --file rs.batch.json --continue-on-error
```

### Manifest

```json
{
  "steps": [
    {
      "type": "import-asset",
      "studio": "Snipe a Slime!",
      "file": "assets/crate.obj",
      "to": "Workspace",
      "name": "Crate"
    },
    {
      "type": "import-image",
      "studio": "Snipe a Slime!",
      "file": "ui/shop.png",
      "to": "StarterGui",
      "kind": "button"
    },
    {
      "type": "validate",
      "studio": "Snipe a Slime!",
      "path": "Workspace.Crate"
    }
  ]
}
```

### Behavior

- Execute steps in order.
- Stop on first error by default.
- Print per-step status and final summary.
- Support variable interpolation for repeated Studio names and roots.
- Support dry-run for operations that can describe planned work.

### Bridge And Plugin Work

- Prefer implementing orchestration in Rust by calling existing command modules directly.
- Do not add a bridge endpoint unless a server-side transactional batch becomes necessary.

### Validation

- Batch with import-image then validate.
- Batch stops on invalid target by default.
- `--continue-on-error` records failure and proceeds.

### Risks

- Cross-step rollback is hard. Be explicit that initial batch mode is ordered execution, not an atomic transaction.

## 9. `rs package`

### Goal

Create a portable folder or archive containing a Studio subtree, scripts, asset references, metadata, and validation results.

### Command

```powershell
rs package --studio "Snipe a Slime!" --path "ServerStorage.SniperSkins" --out ".\packages\sniper-skins.rspkg"
rs package inspect ".\packages\sniper-skins.rspkg"
rs package import --studio "Snipe a Slime!" --file ".\packages\sniper-skins.rspkg" --to ServerStorage
```

### Package Contents

```text
manifest.json
tree/
  instance.json
  Rifle/
    instance.json
    Script.server.lua
assets/
  MeshPart.MeshId.asset.json
validation.json
checksums.json
```

### Behavior

- Build on existing `export`.
- Add package manifest:
  - package version
  - source Studio name/path
  - export timestamp
  - command version
  - validation summary
  - content checksums
- Optional archive format:
  - folder package first
  - zipped `.rspkg` later
- `package import` can deserialize package contents back into Studio.

### Bridge And Plugin Work

- Reuse export and transfer serializer.
- Add package reader/writer in Rust.
- Add import-from-package flow after the package format is stable.

### Validation

- Package a known fixture.
- Inspect prints manifest and summary.
- Reimport package and compare with `diff`.

### Risks

- If packages include only asset references, they are not self-contained for deleted or permission-restricted Roblox assets.
- Reimport semantics must handle conflicts with existing instances.

## 10. `rs import-audio`

### Goal

Import local audio files into Studio as `Sound` instances when the workflow can be supported safely.

### Command

```powershell
rs import-audio --studio "Snipe a Slime!" --file ".\audio\click.wav" --to SoundService --name Click
rs import-audio --studio "Snipe a Slime!" --file ".\audio\theme.ogg" --to ReplicatedStorage.Audio --looped
```

### Behavior

Initial viable version should be one of:

- create `Sound` instances from existing Roblox asset IDs supplied by manifest
- or upload through an authenticated cloud path if the project intentionally supports that
- or create placeholder `Sound` instances with metadata when local-file-to-SoundId is not possible

The command must not pretend local files are playable in Roblox if no real `SoundId` or `AudioContent` path exists.

### Manifest Option

```json
{
  "sounds": [
    {
      "file": "click.wav",
      "assetId": "rbxassetid://1234567890",
      "name": "Click",
      "volume": 0.5
    }
  ]
}
```

### Bridge And Plugin Work

- Add `ImportAudioRequest`.
- Add plugin handler that creates `Sound`, sets `SoundId`, and applies safe properties.
- Keep cloud upload, if added, in a separate explicit implementation with authentication and permission checks.

### Validation

- Asset-ID manifest creates playable `Sound` instances.
- Missing asset ID fails clearly unless placeholder mode is explicitly requested.
- Invalid audio property fails with a useful error.

### Risks

- Roblox audio privacy, moderation, and upload permissions are account/project dependent.
- A fake local-audio import would be worse than no feature. This feature should start with asset-ID manifests unless a verified upload path is available.

## Cross-Cutting Design Requirements

### Shared Output Modes

All new commands should support:

```text
--format text|json
--timeout <seconds>
--studio <selector>
```

Mutating commands should support:

```text
--dry-run
```

where dry-run can be made accurate.

### Error Shape

Bridge errors should continue using the existing envelope:

```json
{ "ok": false, "error": "human-readable", "code": "machine-readable" }
```

CLI errors should:

- identify the selected Studio when relevant
- identify the target path when relevant
- include a short fix hint when the next action is obvious
- never report success when plugin-side mutation failed

### Path Handling

- Use existing Studio selector behavior.
- Resolve instance paths in the plugin so Studio semantics remain authoritative.
- Report ambiguous path segments instead of picking arbitrary duplicate names.

### Serialization And Reference Rules

- Any feature that reads or writes object references must preserve the invariant that every synthetic ID maps to the final live `Instance` before reference properties are applied.
- MeshPart creation must continue to use `CreateMeshPartAsync` before reference resolution.
- Validation and diff should include reference topology so nil welds and stale references become visible.

### Test Strategy

Minimum relevant checks for each feature:

- Rust unit tests for CLI parsing and protocol JSON.
- Luau/plugin smoke path when behavior depends on Studio APIs.
- Live Studio smoke for one happy path and one meaningful failure path.
- `cargo fmt --check`
- `cargo test -p rs`
- `cargo build --release`
- `rojo build default.project.json --output ..\target\plugin-build-check.rbxmx`

## Suggested Implementation Order

1. `validate`
2. `repair-tool` / `wire-tool`
3. `diff`
4. `sync-folder`
5. `import-ui-pack`
6. `snapshot`
7. `create`
8. `batch`
9. `package`
10. `import-audio`

This order front-loads reliability and debuggability. It also lets later features reuse the same validation, diffing, path resolution, and import infrastructure instead of adding isolated one-off commands.
