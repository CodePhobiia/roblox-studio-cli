local Diagnostics = {}

Diagnostics.Fixes = {
    TOOL_REMOVE_BROKEN_JOINTS = "fix.tool.remove-broken-joints",
    TOOL_SET_PART_PHYSICS = "fix.tool.set-part-physics",
    TOOL_UNANCHOR_PARTS = "fix.tool.unanchor-parts",
    TOOL_WELD_DISCONNECTED_PARTS = "fix.tool.weld-disconnected-parts"
}

Diagnostics.Rules = {
    ["asset.animation.missing"] = {
        description = "Animation has an empty AnimationId"
    },
    ["asset.editable.large-risk"] = {
        description = "Editable asset may be too large or local-only for publish"
    },
    ["asset.image.missing"] = {
        description = "Image-like asset reference is empty"
    },
    ["asset.mesh.missing"] = {
        description = "Mesh asset reference is empty"
    },
    ["asset.private-risk"] = {
        description = "Asset may be private, moderated, or inaccessible to another creator"
    },
    ["asset.sound.missing"] = {
        description = "Sound has an empty SoundId"
    },
    ["ownership.unowned"] = {
        description = "Instance is not marked as rs-owned"
    },
    ["path.duplicate-name"] = {
        description = "Siblings share the same name"
    },
    ["ref.external"] = {
        description = "Reference points outside the validated subtree"
    },
    ["ref.missing"] = {
        description = "Object reference property is nil"
    },
    ["tool.handle.ambiguous"] = {
        description = "Tool has multiple direct Handle children"
    },
    ["tool.handle.missing"] = {
        description = "Tool requires a BasePart child named Handle"
    },
    ["tool.handle.optional"] = {
        description = "Tool has RequiresHandle=false and no Handle"
    },
    ["tool.joint.broken"] = {
        description = "Tool rigid joint has missing or invalid endpoints",
        fixId = Diagnostics.Fixes.TOOL_REMOVE_BROKEN_JOINTS
    },
    ["tool.part.anchored"] = {
        description = "Tool BasePart is anchored",
        fixId = Diagnostics.Fixes.TOOL_UNANCHOR_PARTS
    },
    ["tool.part.collision"] = {
        description = "Tool non-handle BasePart collides by default",
        fixId = Diagnostics.Fixes.TOOL_SET_PART_PHYSICS
    },
    ["tool.part.disconnected"] = {
        description = "Tool BasePart is not rigidly connected to Handle",
        fixId = Diagnostics.Fixes.TOOL_WELD_DISCONNECTED_PARTS
    },
    ["tool.parts"] = {
        description = "Tool BasePart count summary"
    }
}

Diagnostics.AssetMissingRules = {
    animation = "asset.animation.missing",
    audio = "asset.sound.missing",
    image = "asset.image.missing",
    mesh = "asset.mesh.missing"
}

Diagnostics.AssetProperties = {
    Animation = {
        AnimationId = { kind = "animation", reportWhenMissing = true }
    },
    Beam = {
        Texture = { kind = "image", validateWhenMissing = true }
    },
    Decal = {
        Texture = { kind = "image", reportWhenMissing = true }
    },
    ImageButton = {
        Image = { kind = "image", fallbackProperties = { "ImageContent" }, reportWhenMissing = true }
    },
    ImageLabel = {
        Image = { kind = "image", fallbackProperties = { "ImageContent" }, reportWhenMissing = true }
    },
    MeshPart = {
        MeshId = { kind = "mesh", reportWhenMissing = true },
        TextureID = { kind = "image", validateWhenMissing = false },
        TextureId = { kind = "image", optionalProperty = true, validateWhenMissing = false }
    },
    ParticleEmitter = {
        Texture = { kind = "image", validateWhenMissing = true }
    },
    ScrollingFrame = {
        BottomImage = { kind = "image", validateWhenMissing = false },
        MidImage = { kind = "image", validateWhenMissing = false },
        TopImage = { kind = "image", validateWhenMissing = false }
    },
    Sound = {
        SoundId = { kind = "audio", reportWhenMissing = true }
    },
    SpecialMesh = {
        MeshId = { kind = "mesh", reportWhenMissing = true },
        TextureId = { kind = "image", validateWhenMissing = false },
        TextureID = { kind = "image", optionalProperty = true, validateWhenMissing = false }
    },
    SurfaceAppearance = {
        ColorMap = { kind = "image", validateWhenMissing = false },
        MetalnessMap = { kind = "image", validateWhenMissing = false },
        NormalMap = { kind = "image", validateWhenMissing = false },
        RoughnessMap = { kind = "image", validateWhenMissing = false }
    },
    Texture = {
        Texture = { kind = "image", reportWhenMissing = true }
    },
    Trail = {
        Texture = { kind = "image", validateWhenMissing = true }
    }
}

function Diagnostics.assetPropertiesFor(instance)
    return Diagnostics.AssetProperties[instance.ClassName]
end

function Diagnostics.missingAssetRule(kind)
    return Diagnostics.AssetMissingRules[kind] or "asset.image.missing"
end

function Diagnostics.ruleInfo(ruleId)
    return Diagnostics.Rules[ruleId]
end

function Diagnostics.fixInfo(fixId)
    for _, id in pairs(Diagnostics.Fixes) do
        if id == fixId then
            return { id = fixId }
        end
    end
    return nil
end

return Diagnostics
