local StudioPath = require(script.Parent.Parent.StudioPath)
local Ownership = require(script.Parent.Parent.Ownership)
local Diagnostics = require(script.Parent.Parent.Diagnostics)

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

local function hasFallbackAsset(instance, meta)
    if type(meta.fallbackProperties) ~= "table" then
        return false
    end
    for _, property in ipairs(meta.fallbackProperties) do
        local value = readProperty(instance, property)
        if value ~= nil and tostring(value) ~= "" then
            return true
        end
    end
    return false
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

local function ruleIdsFor(kind, flags)
    local ids = {}
    local seen = {}
    local function add(id)
        if not seen[id] then
            seen[id] = true
            table.insert(ids, id)
        end
    end
    for _, flag in ipairs(flags) do
        if flag == "missing" or flag == "empty" then
            add(Diagnostics.missingAssetRule(kind))
        elseif flag == "privateRisk" then
            add("asset.private-risk")
        elseif flag == "unowned" then
            add("ownership.unowned")
        elseif flag == "largeEditableRisk" then
            add("asset.editable.large-risk")
        end
    end
    return ids
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
        local props = Diagnostics.assetPropertiesFor(instance)
        if props then
            for property, meta in pairs(props) do
                local value = readProperty(instance, property)
                if isAssetUri(value) or (meta.reportWhenMissing == true and not hasFallbackAsset(instance, meta)) then
                    local flags = flagsFor(instance, value)
                    table.insert(dependencies, {
                        path = instance:GetFullName(),
                        className = instance.ClassName,
                        property = property,
                        kind = meta.kind,
                        value = tostring(value or ""),
                        flags = flags,
                        ruleIds = ruleIdsFor(meta.kind, flags)
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
