# Diagnostics Contract

`rs validate`, `rs deps`, `rs repair-tool`, and `rs publish-check` use stable
rule IDs so human text output and JSON artifacts can be correlated.

## Rule IDs

| ID | Meaning | Safe fix |
| --- | --- | --- |
| `ref.missing` | Object reference property is nil. | none |
| `ref.external` | Object reference points outside the checked subtree. | none |
| `tool.handle.missing` | Tool requires a `BasePart` child named `Handle`. | none |
| `tool.handle.ambiguous` | Tool has multiple direct `Handle` children. | none |
| `tool.joint.broken` | Tool rigid joint has missing or invalid endpoints. | `fix.tool.remove-broken-joints` |
| `tool.part.anchored` | Tool `BasePart` is anchored. | `fix.tool.unanchor-parts` |
| `tool.part.collision` | Non-handle Tool part collides by default. | `fix.tool.set-part-physics` |
| `tool.part.disconnected` | Tool part is not rigidly connected to `Handle`. | `fix.tool.weld-disconnected-parts` |
| `asset.mesh.missing` | Mesh asset reference is empty. | none |
| `asset.image.missing` | Image-like asset reference is empty. | none |
| `asset.sound.missing` | Sound asset reference is empty. | none |
| `asset.animation.missing` | Animation asset reference is empty. | none |
| `asset.private-risk` | Asset may be private or inaccessible to another creator. | none |
| `asset.editable.large-risk` | Editable asset may be too large or local-only for publish. | none |
| `ownership.unowned` | Instance is not marked as `rs` owned. | none |
| `path.duplicate-name` | Siblings share the same name. | none |

`validate --fix` only dispatches `repair-tool` for the explicit `fix.tool.*`
IDs above. Missing handles stay non-fixable from validate because selecting or
renaming a handle requires creator intent.

## Asset Coverage

The shared asset-property matrix covers `Decal`, `Texture`, `Animation`,
`ParticleEmitter`, `Trail`, `Beam`, `SurfaceAppearance`, `Sound`, `MeshPart`,
`SpecialMesh`, `ImageLabel`, `ImageButton`, and `ScrollingFrame` image refs.
`deps` preserves matching `ruleIds` for missing/private/unowned asset risks so
`publish-check` can keep the same IDs in its structured `checks` output.
