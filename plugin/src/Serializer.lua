local CollectionService = game:GetService("CollectionService")
local Allowlist = require(script.Parent.PropertyAllowlist)
local Encoders = require(script.Parent.PropertyEncoders)

local Serializer = {}

local function fullName(instance)
    local ok, value = pcall(function()
        return instance:GetFullName()
    end)
    return ok and value or instance.Name
end

local function readAttributes(instance)
    local ok, attrs = pcall(function()
        return instance:GetAttributes()
    end)
    return ok and attrs or {}
end

local function readTags(instance)
    local ok, tags = pcall(function()
        return CollectionService:GetTags(instance)
    end)
    return ok and tags or {}
end

local BLOCKING_EXTERNAL_REF_PROPERTIES = {
    Part0 = true,
    Part1 = true,
    Attachment0 = true,
    Attachment1 = true
}

local function trackExternalReference(instance, prop, value, externalReferences)
    if type(externalReferences) ~= "table" or typeof(value) ~= "Instance" then
        return
    end
    table.insert(externalReferences, {
        path = fullName(instance),
        className = instance.ClassName,
        property = prop,
        targetPath = fullName(value),
        blocking = BLOCKING_EXTERNAL_REF_PROPERTIES[prop] == true
    })
end

local function readProperties(instance, refs, warnings, externalReferences)
    local properties = {}
    for _, prop in ipairs(Allowlist.forInstance(instance)) do
        local ok, value = pcall(function()
            return instance[prop]
        end)
        if ok then
            local encoded, warning = Encoders.encode(value, refs)
            if encoded ~= nil then
                properties[prop] = encoded
            elseif warning and value ~= nil then
                table.insert(warnings, fullName(instance) .. "." .. prop .. ": " .. warning)
                if string.find(warning, "external instance reference", 1, true) then
                    trackExternalReference(instance, prop, value, externalReferences)
                end
            end
        end
    end
    return properties
end

function Serializer.readInstance(instance, depth, refs)
    local warnings = {}
    local function walk(node, remainingDepth)
        local item = {
            name = node.Name,
            className = node.ClassName,
            fullPath = fullName(node),
            properties = readProperties(node, refs, warnings),
            attributes = readAttributes(node),
            tags = readTags(node),
            children = {}
        }
        if remainingDepth > 0 then
            for _, child in ipairs(node:GetChildren()) do
                table.insert(item.children, walk(child, remainingDepth - 1))
            end
        end
        return item
    end

    local data = walk(instance, depth or 0)
    data.warnings = warnings
    return data
end

function Serializer.serialize(root)
    local refs = {}
    local ordered = {}
    local warnings = {}
    local externalReferences = {}
    local nextId = 0

    local function assign(instance)
        local id = "i" .. tostring(nextId)
        nextId += 1
        refs[instance] = id
        table.insert(ordered, instance)
        for _, child in ipairs(instance:GetChildren()) do
            assign(child)
        end
    end

    assign(root)

    local instances = {}
    for _, instance in ipairs(ordered) do
        local id = refs[instance]
        local children = {}
        for _, child in ipairs(instance:GetChildren()) do
            table.insert(children, refs[child])
        end

        local parentId = nil
        if instance ~= root then
            parentId = refs[instance.Parent]
        end

        instances[id] = {
            className = instance.ClassName,
            parent = parentId,
            properties = readProperties(instance, refs, warnings, externalReferences),
            attributes = readAttributes(instance),
            tags = readTags(instance),
            children = children
        }
    end

    return {
        version = 1,
        root = refs[root],
        instances = instances,
        warnings = warnings,
        externalReferences = externalReferences
    }
end

return Serializer
