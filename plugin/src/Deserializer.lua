local CollectionService = game:GetService("CollectionService")
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

local function createInstance(className, warnings)
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

local function applyProperties(instance, props, idMap, warnings)
    for prop, encoded in pairs(props or {}) do
        if not SKIP_PROPS[prop] then
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
        local instance = createInstance(spec.className, warnings)
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
