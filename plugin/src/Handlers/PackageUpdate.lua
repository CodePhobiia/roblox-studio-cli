local HttpService = game:GetService("HttpService")
local ServerStorage = game:GetService("ServerStorage")
local Deserializer = require(script.Parent.Parent.Deserializer)
local Ownership = require(script.Parent.Parent.Ownership)
local Serializer = require(script.Parent.Parent.Serializer)
local StudioPath = require(script.Parent.Parent.StudioPath)

local function rootNameFromBlob(blob)
    local rootSpec = type(blob) == "table" and type(blob.instances) == "table" and blob.instances[blob.root] or nil
    local props = type(rootSpec) == "table" and rootSpec.properties or nil
    if type(props) == "table" and type(props.Name) == "string" and props.Name ~= "" then
        return props.Name
    end
    return "Imported"
end

local function stableRootSourceId(packageId, blob)
    if type(blob) == "table" and type(blob.root) == "string" then
        return Ownership.stableSourceId(packageId, blob.root, nil)
    end
    return nil
end

local function restoreSnapshot(snapshot, parent)
    local rootName = rootNameFromBlob(snapshot)
    local existing = parent:FindFirstChild(rootName)
    if existing then
        existing:Destroy()
    end
    return Deserializer.deserialize(snapshot, parent)
end

local function stampMap(idMap, packageId)
    for localId, instance in pairs(idMap or {}) do
        Ownership.stamp(instance, Ownership.stableSourceId(packageId, localId, nil), packageId)
    end
end

local function tempParent()
    local folder = Instance.new("Folder")
    folder.Name = "__rsPackageUpdate_" .. HttpService:GenerateGUID(false)
    folder.Parent = ServerStorage
    return folder
end

local function findExisting(parent, packageId, rootSourceId, rootName)
    for _, child in ipairs(parent:GetChildren()) do
        if rootSourceId and child:GetAttribute("rsSourceId") == rootSourceId then
            return child
        end
        if type(packageId) == "string" and packageId ~= "" and child:GetAttribute("rsPackageId") == packageId and child.Name == rootName then
            return child
        end
    end
    return parent:FindFirstChild(rootName)
end

local function findMatchingChild(targetParent, incoming)
    local sourceId = incoming:GetAttribute("rsSourceId")
    if type(sourceId) == "string" and sourceId ~= "" then
        for _, child in ipairs(targetParent:GetChildren()) do
            if child:GetAttribute("rsSourceId") == sourceId then
                return child
            end
        end
    end
    return targetParent:FindFirstChild(incoming.Name)
end

local function recordPath(list, path)
    if type(path) == "string" and path ~= "" then
        table.insert(list, path)
    end
end

local function mergeChildren(target, incoming, options, result)
    for _, child in ipairs(incoming:GetChildren()) do
        local existing = findMatchingChild(target, child)
        if not existing then
            result.created += 1
            local createdPath = target:GetFullName() .. "." .. child.Name
            recordPath(result.changedPaths, createdPath)
            recordPath(result.createdPaths, createdPath)
            if not options.dryRun then
                child.Parent = target
            end
        else
            local existingPath = existing:GetFullName()
            local canMutate, mutateErr = Ownership.canMutate(existing, {
                force = options.force,
                packageId = options.packageId,
                sourceId = child:GetAttribute("rsSourceId")
            })
            if not canMutate then
                if options.mode == "preserve-local" then
                    result.preserved += 1
                    recordPath(result.preservedPaths, existingPath)
                    table.insert(result.warnings, mutateErr)
                    if not options.dryRun then
                        child:Destroy()
                    end
                else
                    result.refused += 1
                    recordPath(result.refusedPaths, existingPath)
                    recordPath(result.unownedPaths, existingPath)
                    table.insert(result.warnings, mutateErr)
                    if not options.dryRun then
                        child:Destroy()
                    end
                end
            elseif options.mode == "preserve-local" then
                mergeChildren(existing, child, options, result)
                result.preserved += 1
                recordPath(result.preservedPaths, existingPath)
                if not options.dryRun then
                    child:Destroy()
                end
            else
                result.replaced += 1
                recordPath(result.changedPaths, existingPath)
                recordPath(result.replacedPaths, existingPath)
                if not options.dryRun then
                    local parent = existing.Parent
                    existing:Destroy()
                    child.Parent = parent
                end
            end
        end
    end
end

local function packageUpdateHandler(payload)
    if type(payload.parentPath) ~= "string" then
        return { ok = false, error = "parentPath missing" }
    end
    if type(payload.blob) ~= "table" then
        return { ok = false, error = "blob missing" }
    end
    if type(payload.packageId) ~= "string" or payload.packageId == "" then
        return { ok = false, error = "packageId missing" }
    end
    local parent, err = StudioPath.resolve(payload.parentPath)
    if not parent then
        return { ok = false, error = err }
    end
    local mode = tostring(payload.mode or "owned-only")
    if mode ~= "owned-only" and mode ~= "preserve-local" and mode ~= "replace-owned" and mode ~= "conflict-report" then
        return { ok = false, error = "unknown package update mode: " .. mode }
    end

    local dryRun = payload.dryRun == true or mode == "conflict-report"
    local rootName = rootNameFromBlob(payload.blob)
    local rootSourceId = stableRootSourceId(payload.packageId, payload.blob)
    local existing = findExisting(parent, payload.packageId, rootSourceId, rootName)
    local result = {
        rootPath = existing and existing:GetFullName() or parent:GetFullName() .. "." .. rootName,
        mode = mode,
        dryRun = dryRun,
        packageId = payload.packageId,
        existing = existing ~= nil,
        owned = existing and Ownership.isOwned(existing, rootSourceId, payload.packageId) or false,
        created = 0,
        replaced = 0,
        preserved = 0,
        refused = 0,
        changedPaths = {},
        createdPaths = {},
        replacedPaths = {},
        preservedPaths = {},
        refusedPaths = {},
        unownedPaths = {},
        rollbackAvailable = existing ~= nil,
        snapshotRecorded = false,
        warnings = {}
    }

    local temp = tempParent()
    local imported, warningsOrErr, idMap = Deserializer.deserialize(payload.blob, temp)
    if not imported then
        temp:Destroy()
        return { ok = false, error = warningsOrErr }
    end
    stampMap(idMap, payload.packageId)
    for _, warning in ipairs(warningsOrErr or {}) do
        table.insert(result.warnings, warning)
    end

    local rootRefused = false
    if existing and not dryRun then
        result.snapshotBefore = Serializer.serialize(existing)
        result.restoreParentPath = parent:GetFullName()
        result.snapshotRecorded = true
    end

    if existing then
        local canMutate, mutateErr = Ownership.canMutate(existing, {
            force = payload.force == true,
            packageId = payload.packageId,
            sourceId = rootSourceId
        })
        if not canMutate and mode ~= "preserve-local" then
            rootRefused = true
            result.refused += 1
            recordPath(result.refusedPaths, existing:GetFullName())
            recordPath(result.unownedPaths, existing:GetFullName())
            table.insert(result.warnings, mutateErr)
        end
    end

    if result.refused == 0 or (dryRun and not (rootRefused and mode == "replace-owned")) then
        if not existing then
            result.created += 1
            local createdPath = parent:GetFullName() .. "." .. imported.Name
            recordPath(result.changedPaths, createdPath)
            recordPath(result.createdPaths, createdPath)
            if not dryRun then
                imported.Parent = parent
            end
            result.rootPath = parent:GetFullName() .. "." .. imported.Name
        elseif mode == "replace-owned" then
            result.replaced += 1
            recordPath(result.changedPaths, existing:GetFullName())
            recordPath(result.replacedPaths, existing:GetFullName())
            if not dryRun then
                existing:Destroy()
                imported.Parent = parent
            end
            result.rootPath = parent:GetFullName() .. "." .. imported.Name
        else
            mergeChildren(existing, imported, {
                dryRun = dryRun,
                force = payload.force == true,
                packageId = payload.packageId,
                mode = mode
            }, result)
            result.rootPath = existing:GetFullName()
        end
    end

    temp:Destroy()
    if not dryRun and result.refused > 0 and result.snapshotBefore then
        local restored, restoreWarningsOrErr = restoreSnapshot(result.snapshotBefore, parent)
        result.rolledBack = restored ~= nil
        if restored then
            result.rootPath = restored:GetFullName()
            table.insert(result.warnings, "rolled back package update because one or more operations were refused")
            for _, warning in ipairs(restoreWarningsOrErr or {}) do
                table.insert(result.warnings, "rollback: " .. tostring(warning))
            end
        else
            table.insert(result.warnings, "rollback failed: " .. tostring(restoreWarningsOrErr))
        end
    end
    return { ok = true, data = result }
end

return packageUpdateHandler
