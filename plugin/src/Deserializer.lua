local CollectionService = game:GetService("CollectionService")
local AssetService = game:GetService("AssetService")
local Encoders = require(script.Parent.PropertyEncoders)

local Deserializer = {}

local SKIP_PROPS = {
    Parent = true,
    ClassName = true
}

local function destroyCreated(created)
    for i = #created, 1, -1 do
        pcall(function()
            created[i]:Destroy()
        end)
    end
end

local function decodeCollisionFidelity(encoded)
    local decoded = nil
    if encoded ~= nil then
        local ok, result = pcall(function()
            return Encoders.decode(encoded, {})
        end)
        if ok then
            decoded = result
        end
    end
    if typeof(decoded) == "EnumItem" and decoded.EnumType == Enum.CollisionFidelity then
        return decoded
    end
    return Enum.CollisionFidelity.Default
end

local function createMeshPart(spec, warnings)
    local props = spec.properties or {}
    local meshId = props.MeshId
    if type(meshId) == "string" and meshId ~= "" and Content and Content.fromUri then
        local ok, meshPartOrErr = pcall(function()
            return AssetService:CreateMeshPartAsync(Content.fromUri(meshId), {
                CollisionFidelity = decodeCollisionFidelity(props.CollisionFidelity)
            })
        end)
        if ok and meshPartOrErr then
            return meshPartOrErr
        end
        table.insert(warnings, "could not create MeshPart from " .. meshId .. ": " .. tostring(meshPartOrErr))
    end

    local ok, fallback = pcall(function()
        return Instance.new("MeshPart")
    end)
    if ok then
        return fallback
    end
    table.insert(warnings, "could not create class 'MeshPart'; using Part")
    return Instance.new("Part")
end

local function createInstance(spec, warnings)
    local className = spec.className
    if className == "MeshPart" then
        return createMeshPart(spec, warnings)
    end

    local ok, instance = pcall(function()
        return Instance.new(className)
    end)
    if ok then
        return instance
    end
    table.insert(warnings, "could not create class '" .. tostring(className) .. "'; using Folder")
    local fallback = Instance.new("Folder")
    fallback:SetAttribute("rsOriginalClassName", tostring(className))
    return fallback
end

local function applyAttributes(instance, attrs, warnings)
    for key, value in pairs(attrs or {}) do
        local ok, err = pcall(function()
            instance:SetAttribute(key, value)
        end)
        if not ok then
            table.insert(warnings, instance:GetFullName() .. " attribute " .. tostring(key) .. ": " .. tostring(err))
        end
    end
end

local function applyTags(instance, tags, warnings)
    for _, tag in ipairs(tags or {}) do
        local ok, err = pcall(function()
            CollectionService:AddTag(instance, tag)
        end)
        if not ok then
            table.insert(warnings, instance:GetFullName() .. " tag " .. tostring(tag) .. ": " .. tostring(err))
        end
    end
end

local function applyOneProperty(instance, prop, encoded, idMap, warnings)
    if SKIP_PROPS[prop] then
        return
    end
    if instance:IsA("MeshPart") and (prop == "MeshId" or prop == "MeshContent") then
        return
    end

    local decoded, decodeWarning = Encoders.decode(encoded, idMap)
    if decodeWarning then
        table.insert(warnings, instance:GetFullName() .. "." .. prop .. ": " .. decodeWarning)
    end
    if decoded ~= nil then
        local ok, err = pcall(function()
            instance[prop] = decoded
        end)
        if not ok then
            table.insert(warnings, instance:GetFullName() .. "." .. prop .. ": " .. tostring(err))
        end
    end
end

local function applyProperties(instance, props, idMap, warnings)
    props = props or {}
    local applied = {}

    for _, prop in ipairs({ "Name", "BrickColor" }) do
        if props[prop] ~= nil then
            applyOneProperty(instance, prop, props[prop], idMap, warnings)
            applied[prop] = true
        end
    end

    for prop, encoded in pairs(props) do
        if not applied[prop] and prop ~= "Color" then
            applyOneProperty(instance, prop, encoded, idMap, warnings)
        end
    end

    if props.Color ~= nil then
        applyOneProperty(instance, "Color", props.Color, idMap, warnings)
    end
end

function Deserializer.deserialize(blob, parent)
    if type(blob) ~= "table" or blob.version ~= 1 or type(blob.instances) ~= "table" or type(blob.root) ~= "string" then
        return nil, "invalid transfer blob"
    end

    local warnings = {}
    local idMap = {}
    local created = {}

    for id, spec in pairs(blob.instances) do
        if type(spec) ~= "table" then
            destroyCreated(created)
            return nil, "invalid instance spec for " .. tostring(id)
        end
        local instance = createInstance(spec, warnings)
        local name = spec.properties and spec.properties.Name
        if type(name) == "string" then
            pcall(function()
                instance.Name = name
            end)
        end
        idMap[id] = instance
        table.insert(created, instance)
    end

    local root = idMap[blob.root]
    if not root then
        destroyCreated(created)
        return nil, "root instance missing from transfer blob"
    end

    for id, spec in pairs(blob.instances) do
        local instance = idMap[id]
        local targetParent = parent
        if spec.parent ~= nil then
            targetParent = idMap[spec.parent]
        end
        if not targetParent then
            destroyCreated(created)
            return nil, "missing parent reference for " .. tostring(id)
        end
        instance.Parent = targetParent
    end

    for id, spec in pairs(blob.instances) do
        local instance = idMap[id]
        applyProperties(instance, spec.properties, idMap, warnings)
        applyAttributes(instance, spec.attributes, warnings)
        applyTags(instance, spec.tags, warnings)
    end

    for _, warning in ipairs(blob.warnings or {}) do
        table.insert(warnings, "source: " .. tostring(warning))
    end

    return root, warnings
end

return Deserializer
