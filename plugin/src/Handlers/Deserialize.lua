local Deserializer = require(script.Parent.Parent.Deserializer)
local Ownership = require(script.Parent.Parent.Ownership)

local function splitPath(path)
    local parts = {}
    for part in string.gmatch(path, "[^%.]+") do
        table.insert(parts, part)
    end
    return parts
end

local function resolvePath(path)
    local node = game
    for i, part in ipairs(splitPath(path)) do
        if i == 1 then
            local service = game:FindFirstChild(part)
            if not service then
                local ok, result = pcall(function()
                    return game:GetService(part)
                end)
                service = ok and result or nil
            end
            node = service
        else
            node = node and node:FindFirstChild(part)
        end
        if not node then
            return nil
        end
    end
    return node
end

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

local function deserializeHandler(payload)
    if type(payload.parentPath) ~= "string" then
        return { ok = false, error = "parentPath missing" }
    end
    if type(payload.blob) ~= "table" then
        return { ok = false, error = "blob missing" }
    end

    local parent = resolvePath(payload.parentPath)
    if not parent then
        return { ok = false, error = "parent path not found: " .. payload.parentPath }
    end

    local conflictMode = tostring(payload.conflictMode or "allow")
    local rootName = rootNameFromBlob(payload.blob)
    local existing = parent:FindFirstChild(rootName)
    local existingBackup = nil
    if payload.dryRun == true then
        return {
            ok = true,
            data = {
                rootPath = parent:GetFullName() .. "." .. rootName,
                dryRun = true,
                conflict = existing ~= nil,
                conflictMode = conflictMode,
                warnings = existing and { "existing child named " .. rootName .. " would trigger conflict mode " .. conflictMode } or {}
            }
        }
    end
    if existing then
        if conflictMode == "fail" then
            return { ok = false, error = "destination already has child named " .. rootName }
        elseif conflictMode == "replace" then
            if payload.rollbackOnError == true then
                local okClone, cloneOrErr = pcall(function()
                    return existing:Clone()
                end)
                if okClone and cloneOrErr then
                    existingBackup = cloneOrErr
                end
            end
            existing:Destroy()
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
    if existingBackup then
        existingBackup:Destroy()
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
    return { ok = true, data = { rootPath = root:GetFullName(), warnings = warningsOrErr or {} } }
end

return deserializeHandler
