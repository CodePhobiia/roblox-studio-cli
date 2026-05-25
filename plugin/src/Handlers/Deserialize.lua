local Deserializer = require(script.Parent.Parent.Deserializer)
local Ownership = require(script.Parent.Parent.Ownership)
local Inspector = require(script.Parent.Parent.Inspector)
local StudioPath = require(script.Parent.Parent.StudioPath)

local function rootNameFromBlob(blob)
    local rootSpec = blob.instances and blob.instances[blob.root]
    local props = type(rootSpec) == "table" and rootSpec.properties or nil
    if type(props) == "table" and type(props.Name) == "string" and props.Name ~= "" then
        return props.Name
    end
    return "Imported"
end

local function uniqueName(parent, baseName)
    local candidate = baseName
    local index = 2
    while parent:FindFirstChild(candidate) do
        candidate = baseName .. "_" .. tostring(index)
        index += 1
    end
    return candidate
end

local function mergeInto(existing, imported)
    for _, child in ipairs(imported:GetChildren()) do
        child.Parent = existing
    end
    for key, value in pairs(imported:GetAttributes()) do
        pcall(function()
            existing:SetAttribute(key, value)
        end)
    end
    imported:Destroy()
    return existing
end

local function blockingExternalReferences(blob)
    local blocked = {}
    for _, ref in ipairs(type(blob.externalReferences) == "table" and blob.externalReferences or {}) do
        if type(ref) == "table" and ref.blocking == true then
            table.insert(blocked, ref)
        end
    end
    return blocked
end

local function summarizeExternalReferences(blocked)
    local parts = {}
    for _, ref in ipairs(blocked) do
        table.insert(parts, tostring(ref.path) .. "." .. tostring(ref.property) .. " -> " .. tostring(ref.targetPath))
        if #parts >= 5 then
            break
        end
    end
    if #blocked > #parts then
        table.insert(parts, "... " .. tostring(#blocked - #parts) .. " more")
    end
    return table.concat(parts, "; ")
end

local function validationFailed(validation)
    local summary = type(validation) == "table" and validation.summary or nil
    return type(summary) == "table" and tonumber(summary.fail) and tonumber(summary.fail) > 0
end

local function rollbackCreated(root, parent, backup)
    if root then
        pcall(function()
            root:Destroy()
        end)
    end
    if backup then
        backup.Parent = parent
    end
end

local function deserializeHandler(payload)
    if type(payload.parentPath) ~= "string" then
        return { ok = false, error = "parentPath missing" }
    end
    if type(payload.blob) ~= "table" then
        return { ok = false, error = "blob missing" }
    end

    local parent, resolveErr = StudioPath.resolve(payload.parentPath)
    if not parent then
        return { ok = false, error = resolveErr or ("parent path not found: " .. payload.parentPath) }
    end

    local conflictMode = tostring(payload.conflictMode or "allow")
    local rootName = rootNameFromBlob(payload.blob)
    local existing = parent:FindFirstChild(rootName)
    local existingBackup = nil
    local externalFailures = blockingExternalReferences(payload.blob)
    if payload.failOnExternalRefs == true and #externalFailures > 0 then
        return {
            ok = false,
            error = "transfer has external rigid references outside the selected root: " .. summarizeExternalReferences(externalFailures)
        }
    end
    if payload.dryRun == true then
        return {
            ok = true,
            data = {
                rootPath = parent:GetFullName() .. "." .. rootName,
                dryRun = true,
                conflict = existing ~= nil,
                conflictMode = conflictMode,
                externalReferences = payload.blob.externalReferences or {},
                warnings = existing and { "existing child named " .. rootName .. " would trigger conflict mode " .. conflictMode } or {}
            }
        }
    end
    if existing then
        if conflictMode == "fail" then
            return { ok = false, error = "destination already has child named " .. rootName }
        elseif conflictMode == "replace" or conflictMode == "merge" then
            if payload.rollbackOnError == true then
                local okClone, cloneOrErr = pcall(function()
                    return existing:Clone()
                end)
                if okClone and cloneOrErr then
                    existingBackup = cloneOrErr
                end
            end
            if conflictMode == "replace" then
                existing:Destroy()
            end
        elseif conflictMode ~= "rename" and conflictMode ~= "merge" and conflictMode ~= "allow" then
            return { ok = false, error = "unknown conflictMode: " .. conflictMode }
        end
    end

    local root, warningsOrErr, idMap = Deserializer.deserialize(payload.blob, parent)
    if not root then
        if existingBackup then
            existingBackup.Parent = parent
        end
        return { ok = false, error = warningsOrErr }
    end
    if existing and conflictMode == "rename" then
        root.Name = uniqueName(parent, rootName)
    elseif existing and conflictMode == "merge" then
        root = mergeInto(existing, root)
    end
    local sourceId = Ownership.sourceId(payload, "package")
    if type(idMap) == "table" then
        for localId, instance in pairs(idMap) do
            Ownership.stamp(instance, Ownership.stableSourceId(payload.packageId, localId, sourceId), payload.packageId)
        end
    else
        Ownership.stamp(root, sourceId, payload.packageId)
        for _, descendant in ipairs(root:GetDescendants()) do
            Ownership.stamp(descendant, sourceId, payload.packageId)
        end
    end

    local validation = nil
    if type(payload.validateRules) == "table" and #payload.validateRules > 0 then
        validation = Inspector.validate(root, payload.validateRules)
        if payload.failOnValidationFailure == true and validationFailed(validation) then
            if payload.rollbackOnError == true then
                rollbackCreated(root, parent, existingBackup)
            elseif existingBackup then
                existingBackup:Destroy()
            end
            return {
                ok = false,
                error = "post-deserialize validation failed with " .. tostring(validation.summary.fail) .. " failing diagnostic(s)"
            }
        end
    end

    if existingBackup then
        existingBackup:Destroy()
    end
    return {
        ok = true,
        data = {
            rootPath = root:GetFullName(),
            warnings = warningsOrErr or {},
            validation = validation,
            externalReferences = payload.blob.externalReferences or {}
        }
    }
end

return deserializeHandler
