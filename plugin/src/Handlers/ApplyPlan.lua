local CollectionService = game:GetService("CollectionService")
local StudioPath = require(script.Parent.Parent.StudioPath)
local Encoders = require(script.Parent.Parent.PropertyEncoders)
local Ownership = require(script.Parent.Parent.Ownership)
local Serializer = require(script.Parent.Parent.Serializer)
local Deserializer = require(script.Parent.Parent.Deserializer)

local SCRIPT_CLASSES = {
    Script = true,
    LocalScript = true,
    ModuleScript = true
}

local function splitDot(path)
    local parts = {}
    for part in string.gmatch(tostring(path or ""), "[^%.]+") do
        table.insert(parts, part)
    end
    return parts
end

local function parentPathOf(path)
    local parts = splitDot(path)
    if #parts <= 1 then
        return "game"
    end
    table.remove(parts, #parts)
    return table.concat(parts, ".")
end

local function rootNameFromBlob(blob)
    local rootSpec = type(blob) == "table" and type(blob.instances) == "table" and blob.instances[blob.root] or nil
    local props = type(rootSpec) == "table" and rootSpec.properties or nil
    if type(props) == "table" and type(props.Name) == "string" and props.Name ~= "" then
        return props.Name
    end
    return "Restored"
end

local function restoreSnapshot(snapshot, restoreParentPath)
    local parent, err = StudioPath.resolve(restoreParentPath)
    if not parent then
        return nil, err
    end
    local existing = parent:FindFirstChild(rootNameFromBlob(snapshot))
    if existing then
        existing:Destroy()
    end
    local root, warningsOrErr = Deserializer.deserialize(snapshot, parent)
    if not root then
        return nil, warningsOrErr
    end
    return root, warningsOrErr
end

local function resolveRelative(root, relative)
    if relative == "." or relative == "" or relative == nil then
        return root
    end
    local node = root
    for _, part in ipairs(splitDot(relative)) do
        node = node and node:FindFirstChild(part)
        if not node then
            return nil
        end
    end
    return node
end

local function parentAndName(root, relative)
    local parts = splitDot(relative)
    local name = table.remove(parts, #parts)
    if not name or name == "" then
        return nil, nil
    end
    if #parts == 0 then
        return root, name
    end
    return resolveRelative(root, table.concat(parts, ".")), name
end

local function toSet(values)
    local set = {}
    if type(values) == "table" then
        for _, value in ipairs(values) do
            set[string.lower(tostring(value))] = true
        end
    end
    return set
end

local function changeAllowed(change, onlySet)
    if not next(onlySet) then
        return true
    end
    return onlySet[string.lower(tostring(change.kind or ""))] == true
end

local function excludesScripts(excludeSet)
    return excludeSet.scripts == true or excludeSet.script == true
end

local function classExcluded(className, excludeSet)
    if excludeSet[string.lower(tostring(className or ""))] == true then
        return true
    end
    return excludesScripts(excludeSet) and SCRIPT_CLASSES[className] == true
end

local function propertyExcluded(instance, property, excludeSet)
    if not excludesScripts(excludeSet) then
        return false
    end
    return instance and SCRIPT_CLASSES[instance.ClassName] == true and tostring(property or "") == "Source"
end

local function decodeValue(value)
    if type(value) == "table" and value[1] == "InstancePath" and type(value[2]) == "string" then
        local instance = StudioPath.resolve(value[2])
        return instance
    end
    local decoded = nil
    local warning = nil
    local ok, resultOrErr = pcall(function()
        return Encoders.decode(value, {})
    end)
    if ok then
        decoded = resultOrErr
    else
        warning = tostring(resultOrErr)
    end
    return decoded, warning
end

local function setTags(instance, encoded)
    local desired = {}
    if type(encoded) == "table" then
        for _, tag in ipairs(encoded) do
            desired[tostring(tag)] = true
        end
    end
    for _, tag in ipairs(CollectionService:GetTags(instance)) do
        if not desired[tag] then
            CollectionService:RemoveTag(instance, tag)
        end
    end
    for tag in pairs(desired) do
        CollectionService:AddTag(instance, tag)
    end
end

local function applySet(root, rootPath, change, options, result)
    local instance = resolveRelative(root, change.path)
    if not instance then
        result.refused += 1
        table.insert(result.warnings, "path not found: " .. rootPath .. "." .. tostring(change.path))
        return
    end
    local scope, name = string.match(tostring(change.property or ""), "^([^.]+)%.(.+)$")
    if not scope and change.property == "tags" then
        scope = "tags"
    end
    if not scope then
        result.skipped += 1
        table.insert(result.warnings, "cannot apply class/topology change at " .. instance:GetFullName())
        return
    end
    if propertyExcluded(instance, name, options.excludeSet) then
        result.skipped += 1
        return
    end
    local canMutate, mutateErr = Ownership.canMutate(instance, { force = options.force })
    if not canMutate then
        result.refused += 1
        table.insert(result.warnings, mutateErr)
        return
    end
    if options.dryRun then
        result.applied += 1
        table.insert(result.changedPaths, instance:GetFullName())
        return
    end
    local ok, err = pcall(function()
        if scope == "properties" then
            local decoded = decodeValue(change.after)
            instance[name] = decoded
        elseif scope == "attributes" then
            instance:SetAttribute(name, change.after)
        elseif scope == "tags" then
            setTags(instance, change.after)
        else
            error("unsupported property scope: " .. tostring(scope))
        end
    end)
    if ok then
        result.applied += 1
        table.insert(result.changedPaths, instance:GetFullName())
    else
        result.refused += 1
        table.insert(result.warnings, instance:GetFullName() .. ": " .. tostring(err))
    end
end

local function applyCreate(root, change, options, result)
    local className = type(change.after) == "string" and change.after or "Folder"
    if classExcluded(className, options.excludeSet) then
        result.skipped += 1
        return
    end
    local parent, name = parentAndName(root, change.path)
    if not parent then
        result.refused += 1
        table.insert(result.warnings, "parent not found for create: " .. tostring(change.path))
        return
    end
    if parent:FindFirstChild(name) then
        result.skipped += 1
        table.insert(result.warnings, "create skipped because instance already exists: " .. parent:GetFullName() .. "." .. name)
        return
    end
    if options.dryRun then
        result.applied += 1
        table.insert(result.changedPaths, parent:GetFullName() .. "." .. name)
        return
    end
    local ok, instanceOrErr = pcall(function()
        return Instance.new(className)
    end)
    if not ok or not instanceOrErr then
        result.refused += 1
        table.insert(result.warnings, "could not create class '" .. className .. "': " .. tostring(instanceOrErr))
        return
    end
    instanceOrErr.Name = name
    Ownership.stamp(instanceOrErr, options.sourcePrefix .. ":" .. tostring(change.path or instanceOrErr.Name))
    instanceOrErr.Parent = parent
    result.applied += 1
    table.insert(result.changedPaths, instanceOrErr:GetFullName())
end

local function applyDelete(root, change, options, result)
    local instance = resolveRelative(root, change.path)
    if not instance then
        result.skipped += 1
        return
    end
    local canMutate, mutateErr = Ownership.canMutate(instance, { force = options.force })
    if not canMutate then
        result.refused += 1
        table.insert(result.warnings, mutateErr)
        return
    end
    if options.dryRun then
        result.applied += 1
        table.insert(result.changedPaths, instance:GetFullName())
        return
    end
    table.insert(result.changedPaths, instance:GetFullName())
    instance:Destroy()
    result.applied += 1
end

local function applyPlanHandler(payload)
    if type(payload.rootPath) ~= "string" then
        return { ok = false, error = "rootPath missing" }
    end
    if type(payload.plan) ~= "table" then
        return { ok = false, error = "plan missing" }
    end
    if payload.dryRun ~= true and payload.approved ~= true then
        return { ok = false, error = "apply-plan requires approval; pass --yes from the CLI" }
    end
    if payload.dryRun ~= true and payload.plan.safeToApply == false then
        return { ok = false, error = "fix plan has conflicts; refusing to apply unsafe plan" }
    end
    local root, err = StudioPath.resolve(payload.rootPath)
    if not root then
        return { ok = false, error = err }
    end
    local changes = type(payload.plan.changes) == "table" and payload.plan.changes or {}
    local result = {
        rootPath = root:GetFullName(),
        dryRun = payload.dryRun == true,
        applied = 0,
        skipped = 0,
        refused = 0,
        changedPaths = {},
        warnings = {}
    }
    if not result.dryRun then
        result.snapshotBefore = Serializer.serialize(root)
        result.restoreParentPath = parentPathOf(payload.rootPath)
    end
    local options = {
        dryRun = result.dryRun,
        force = payload.force == true,
        onlySet = toSet(payload.only),
        excludeSet = toSet(payload.exclude),
        sourcePrefix = "apply-plan:" .. root:GetFullName()
    }
    for _, change in ipairs(changes) do
        if not changeAllowed(change, options.onlySet) then
            result.skipped += 1
        elseif change.kind == "added" then
            applyCreate(root, change, options, result)
        elseif change.kind == "deleted" then
            applyDelete(root, change, options, result)
        elseif change.kind == "modified" or change.kind == "reference" then
            applySet(root, payload.rootPath, change, options, result)
        else
            result.skipped += 1
            table.insert(result.warnings, "unknown change kind: " .. tostring(change.kind))
        end
    end
    if not result.dryRun and result.refused > 0 and result.snapshotBefore then
        local restored, restoreWarningsOrErr = restoreSnapshot(result.snapshotBefore, result.restoreParentPath)
        result.rolledBack = restored ~= nil
        if restored then
            result.rootPath = restored:GetFullName()
            table.insert(result.warnings, "rolled back apply-plan because one or more operations were refused")
            for _, warning in ipairs(restoreWarningsOrErr or {}) do
                table.insert(result.warnings, "rollback: " .. tostring(warning))
            end
        else
            table.insert(result.warnings, "rollback failed: " .. tostring(restoreWarningsOrErr))
        end
    end
    return { ok = true, data = result }
end

return applyPlanHandler
