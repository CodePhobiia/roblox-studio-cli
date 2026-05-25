local StudioPath = require(script.Parent.Parent.StudioPath)
local Ownership = require(script.Parent.Parent.Ownership)

local ASSET_PROPERTIES = {
    MeshPart = { MeshId = "mesh", TextureID = "image", TextureId = "image" },
    SpecialMesh = { MeshId = "mesh", TextureId = "image", TextureID = "image" },
    Decal = { Texture = "image" },
    Texture = { Texture = "image" },
    ImageLabel = { Image = "image" },
    ImageButton = { Image = "image" },
    ScrollingFrame = { BottomImage = "image", MidImage = "image", TopImage = "image" },
    Sound = { SoundId = "audio" },
    Animation = { AnimationId = "animation" },
    ParticleEmitter = { Texture = "image" },
    Trail = { Texture = "image" },
    Beam = { Texture = "image" },
    SurfaceAppearance = {
        ColorMap = "image",
        MetalnessMap = "image",
        NormalMap = "image",
        RoughnessMap = "image"
    }
}

local SCRIPT_CLASSES = {
    Script = true,
    LocalScript = true,
    ModuleScript = true
}

local REMOTE_CLASSES = {
    RemoteEvent = true,
    RemoteFunction = true,
    UnreliableRemoteEvent = true,
    BindableEvent = true,
    BindableFunction = true
}

local function descendantsInclusive(root)
    local items = { root }
    for _, descendant in ipairs(root:GetDescendants()) do
        table.insert(items, descendant)
    end
    return items
end

local function readProperty(instance, property)
    local ok, value = pcall(function()
        return instance[property]
    end)
    if ok then
        return value
    end
    return nil
end

local function isAssetUri(value)
    return type(value) == "string" and value ~= ""
end

local function flagsFor(instance, value)
    local flags = {}
    if type(value) ~= "string" or value == "" then
        table.insert(flags, "missing")
        table.insert(flags, "empty")
    elseif string.match(value, "^rbxassetid://%d+") or string.match(value, "^%d+$") then
        table.insert(flags, "privateRisk")
    end
    if not Ownership.isOwned(instance) then
        table.insert(flags, "unowned")
    end
    if instance.ClassName == "EditableImage" or instance.ClassName == "EditableMesh" then
        table.insert(flags, "largeEditableRisk")
    end
    return flags
end

local function depsHandler(payload)
    if type(payload.path) ~= "string" then
        return { ok = false, error = "path missing" }
    end
    local root, err = StudioPath.resolve(payload.path)
    if not root then
        return { ok = false, error = err }
    end
    local dependencies = {}
    local scripts = {}
    local remotes = {}
    local unowned = {}
    local warnings = {}

    for _, instance in ipairs(descendantsInclusive(root)) do
        if not Ownership.isOwned(instance) then
            table.insert(unowned, instance:GetFullName())
        end
        if SCRIPT_CLASSES[instance.ClassName] then
            table.insert(scripts, instance:GetFullName())
        end
        if REMOTE_CLASSES[instance.ClassName] then
            table.insert(remotes, instance:GetFullName())
        end
        if instance.ClassName == "EditableImage" or instance.ClassName == "EditableMesh" then
            table.insert(warnings, "large editable asset risk: " .. instance:GetFullName())
        end
        local props = ASSET_PROPERTIES[instance.ClassName]
        if props then
            for property, kind in pairs(props) do
                local value = readProperty(instance, property)
                if isAssetUri(value) or property == "MeshId" or property == "SoundId" or property == "Image" or property == "AnimationId" then
                    table.insert(dependencies, {
                        path = instance:GetFullName(),
                        className = instance.ClassName,
                        property = property,
                        kind = kind,
                        value = tostring(value or ""),
                        flags = flagsFor(instance, value)
                    })
                end
            end
        end
    end

    return {
        ok = true,
        data = {
            rootPath = root:GetFullName(),
            dependencies = dependencies,
            scripts = scripts,
            remotes = remotes,
            unownedInstances = unowned,
            warnings = warnings
        }
    }
end

return depsHandler
