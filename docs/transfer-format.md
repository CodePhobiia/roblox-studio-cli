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

The deserializer creates every instance first, parents the tree, then applies properties so `Weld.Part0`, `Weld.Part1`, `Motor6D.Part0`, `ObjectValue.Value`, and similar references can resolve across the transferred tree.

Supported scalar and structured encodings include primitives, `Vector2`, `Vector3`, `CFrame`, `Color3`, `BrickColor`, `UDim`, `UDim2`, `Rect`, `EnumItem`, `ColorSequence`, `NumberSequence`, `NumberRange`, `PhysicalProperties`, and object references.
