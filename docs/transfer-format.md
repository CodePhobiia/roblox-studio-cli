# Transfer Format

The plugin serializer emits a JSON-compatible blob:

```json
{
  "version": 1,
  "root": "i0",
  "instances": {
    "i0": {
      "className": "Tool",
      "parent": null,
      "properties": { "Name": "SP-R 208" },
      "attributes": {},
      "tags": [],
      "children": ["i1"]
    }
  },
  "warnings": []
}
```

Instance references are encoded as synthetic IDs:

```json
["Ref", "i7"]
```

The deserializer first pre-creates MeshParts with `AssetService:CreateMeshPartAsync`, waits for those creations to finish, then creates every remaining instance into the same ID map. It applies properties before parenting the tree so `Weld.Part0`, `Weld.Part1`, `Motor6D.Part0`, `ObjectValue.Value`, and similar references resolve against the complete transferred subtree.

Supported scalar and structured encodings include primitives, `Vector2`, `Vector3`, `CFrame`, `Color3`, `BrickColor`, `UDim`, `UDim2`, `Rect`, `EnumItem`, `ColorSequence`, `NumberSequence`, `NumberRange`, `PhysicalProperties`, and object references.

Attachment-family instances, including `Bone`, serialize their local transform properties through an inherited `Attachment` allowlist branch. This preserves skinned mesh skeleton offsets during transfer instead of recreating bones at the origin.

When a serialized weld, joint, or attachment constraint points outside the selected source root, the serializer records it in `externalReferences` and omits the unsafe property. `rs transfer` rejects those blocking external refs by default so partial-root transfers do not silently create nil `Part0`, `Part1`, `Attachment0`, or `Attachment1` properties. Transfer a common parent that contains both endpoints, or pass `--allow-external-refs` when the missing link is intentional.

Cross-Studio transfer asks the target plugin to validate `refs,welds,tool` after deserialize. Validation failures make transfer fail; when `--rollback-on-error` is set, the plugin removes the imported root and restores any replaced destination child.

Content-like properties remain Roblox asset URI strings in the blob where the
Studio property accepts URI assignment, including UI `Image`, decal/texture/VFX
`Texture`, mesh `MeshId`, `TextureID`, `TextureId`, sound `SoundId`, animation
`AnimationId`, scrolling-frame `BottomImage`, `MidImage`, `TopImage`, and
`SurfaceAppearance` `ColorMap`, `MetalnessMap`, `NormalMap`, and `RoughnessMap`.
The plugin serializer/exporter/deps surfaces are aligned to this
content-property matrix for these families. `rs package import --rehost-images` and
`rs transfer --rehost-images` can download the CLI-supported image references
through Open Cloud, upload target-owned copies, and rewrite the in-memory blob
before deserialization. Rehost coverage is defined by the Rust rehost matrix;
content IDs outside that matrix are preserved as references.

Package import and update run a local preflight before sending any mutating
Studio request. The CLI requires package manifest version `1`, a non-empty
`packageId`, a readable version `1` `transfer_blob.json`, a known conflict mode,
and clean `checksums.json` coverage with no missing, mismatched, or unexpected
files. Legacy package folders without `packageId` can still be inspected, but
mutation is refused until they are re-exported with current package metadata.
