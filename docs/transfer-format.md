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

Image-like properties remain Roblox asset URI strings in the blob, including `Image`,
`Texture`, `TextureID`, `TextureId`, and scrolling-frame `BottomImage`, `MidImage`, and
`TopImage`. `rs package import --rehost-images` and `rs transfer --rehost-images` can
download those referenced image assets through Open Cloud, upload target-owned copies, and
rewrite the in-memory blob before deserialization.
