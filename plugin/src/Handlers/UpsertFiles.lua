local StudioPath = require(script.Parent.Parent.StudioPath)
local Ownership = require(script.Parent.Parent.Ownership)

local SCRIPT_CLASSES = {
    Script = true,
    LocalScript = true,
    ModuleScript = true
}

local RESERVED_NAMES = {
    CON = true,
    PRN = true,
    AUX = true,
    NUL = true,
    COM1 = true,
    COM2 = true,
    COM3 = true,
    COM4 = true,
    COM5 = true,
    COM6 = true,
    COM7 = true,
    COM8 = true,
    COM9 = true,
    LPT1 = true,
    LPT2 = true,
    LPT3 = true,
    LPT4 = true,
    LPT5 = true,
    LPT6 = true,
    LPT7 = true,
    LPT8 = true,
    LPT9 = true
}

local function isReservedName(segment)
    local stem = string.match(segment, "^[^%.]+") or segment
    return RESERVED_NAMES[string.upper(stem)] == true
end

local function validatePathSegment(segment, originalPath)
    if segment == "" then
        return "path contains an empty segment: " .. tostring(originalPath)
    end
    if segment == "." or segment == ".." then
        return "path contains an ambiguous segment '" .. segment .. "': " .. tostring(originalPath)
    end
    if string.match(segment, "[<>:\"|%?%*%c]") then
        return "path contains an invalid filename character: " .. tostring(originalPath)
    end
    if string.sub(segment, -1) == "." or string.sub(segment, -1) == " " then
        return "path segment must not end with '.' or space: " .. tostring(originalPath)
    end
    if isReservedName(segment) then
        return "path contains a reserved filename segment '" .. segment .. "': " .. tostring(originalPath)
    end
    return nil
end

local function splitRelativePath(path)
    if type(path) ~= "string" or path == "" then
        return nil, "path missing"
    end
    if string.sub(path, 1, 1) == "/" or string.sub(path, 1, 1) == "\\" then
        return nil, "path must be relative: " .. tostring(path)
    end
    if string.match(path, "^%a:") then
        return nil, "path must not use a drive prefix: " .. tostring(path)
    end
    if string.find(path, ":", 1, true) then
        return nil, "path contains an ambiguous ':' segment: " .. tostring(path)
    end

    local normalized = string.gsub(path, "\\", "/")
    if string.sub(normalized, -1) == "/" or string.find(normalized, "//", 1, true) then
        return nil, "path contains an empty segment: " .. tostring(path)
    end

    local parts = {}
    for part in string.gmatch(normalized, "[^/]+") do
        local err = validatePathSegment(part, path)
        if err then
            return nil, err
        end
        table.insert(parts, part)
    end
    if #parts == 0 then
        return nil, "path missing"
    end
    return parts, nil
end

local function sourceMatches(instance, sourceId)
    local existingSourceId = nil
    pcall(function()
        existingSourceId = instance:GetAttribute("rsSourceId")
    end)
    return type(sourceId) == "string" and sourceId ~= "" and existingSourceId == sourceId
end

local function canMutateSynced(instance, sourceId, force)
    if force == true then
        return true, nil
    end
    if sourceMatches(instance, sourceId) then
        return true, nil
    end
    local path = "<unknown>"
    pcall(function()
        path = instance:GetFullName()
    end)
    return false, "refusing to overwrite synced instance from a different source: " .. path .. " (pass --force to override)"
end

local function ensureFolder(parent, name, dryRun, sourceId, force)
    local existing = parent:FindFirstChild(name)
    if existing then
        if not existing:IsA("Folder") then
            return nil, "path segment exists but is not a Folder: " .. existing:GetFullName()
        end
        local isGenerated = false
        pcall(function()
            isGenerated = existing:GetAttribute("rsSyncGenerated") == true or existing:GetAttribute("rsManagedBy") == "rs"
        end)
        if isGenerated then
            local canMutate, mutateErr = canMutateSynced(existing, sourceId, force)
            if not canMutate then
                return nil, mutateErr
            end
        end
        return existing, false
    end
    if dryRun then
        return parent, true
    end
    local folder = Instance.new("Folder")
    folder.Name = name
    folder:SetAttribute("rsSyncGenerated", true)
    Ownership.stamp(folder, sourceId)
    folder.Parent = parent
    return folder, true
end

local function ensureParent(root, relativePath, dryRun, sourceId, force)
    local parts, pathErr = splitRelativePath(relativePath)
    if not parts then
        return nil, pathErr
    end
    local parent = root
    for i = 1, math.max(#parts - 1, 0) do
        local nextParent, err = ensureFolder(parent, parts[i], dryRun, sourceId, force)
        if not nextParent then
            return nil, err
        end
        parent = nextParent
    end
    return parent, nil
end

local function upsertItem(root, item, dryRun, sourceId, force)
    local className = tostring(item.className or "")
    if className == "" then
        return nil, "className missing"
    end
    local name = StudioPath.sanitize(item.name, "Synced")
    local parts, pathErr = splitRelativePath(item.path)
    if not parts then
        return nil, pathErr
    end
    local normalizedPath = table.concat(parts, "/")
    local parent, err = ensureParent(root, item.path, dryRun, sourceId, force)
    if not parent then
        return nil, err
    end

    local existing = parent:FindFirstChild(name)
    local created = false
    local updated = false
    if existing then
        local canMutate, mutateErr = canMutateSynced(existing, sourceId, force)
        if not canMutate then
            return nil, mutateErr
        end
    end
    if existing and existing.ClassName ~= className then
        if dryRun then
            return { path = parent:GetFullName() .. "." .. name, created = false, updated = true, unchanged = false }, nil
        end
        existing:Destroy()
        existing = nil
    end
    local instance = existing
    if not instance then
        created = true
        if not dryRun then
            local ok, newInstanceOrErr = pcall(function()
                return Instance.new(className)
            end)
            if not ok or not newInstanceOrErr then
                return nil, "could not create class '" .. className .. "': " .. tostring(newInstanceOrErr)
            end
            instance = newInstanceOrErr
            instance.Name = name
            instance.Parent = parent
        end
    end

    if instance and SCRIPT_CLASSES[className] and type(item.source) == "string" then
        local current = nil
        pcall(function()
            current = instance.Source
        end)
        if current ~= item.source then
            updated = not created
            if not dryRun then
                instance.Source = item.source
            end
        end
    end

    if instance and not dryRun then
        instance:SetAttribute("rsSyncPath", normalizedPath)
        instance:SetAttribute("rsSyncClassName", className)
        Ownership.stamp(instance, sourceId)
        for key, value in pairs(item.attributes or {}) do
            pcall(function()
                instance:SetAttribute(key, value)
            end)
        end
    end

    local path = instance and instance:GetFullName() or parent:GetFullName() .. "." .. name
    return {
        path = path,
        created = created,
        updated = updated,
        unchanged = not created and not updated
    }, nil
end

local function collectOwned(root)
    local owned = {}
    for _, descendant in ipairs(root:GetDescendants()) do
        local path = descendant:GetAttribute("rsSyncPath")
        if type(path) == "string" and path ~= "" then
            owned[path] = descendant
        end
    end
    return owned
end

local function upsertFilesHandler(payload)
    if type(payload.parentPath) ~= "string" then
        return { ok = false, error = "parentPath missing" }
    end
    if type(payload.items) ~= "table" then
        return { ok = false, error = "items missing" }
    end
    local root, err = StudioPath.resolve(payload.parentPath)
    if not root then
        return { ok = false, error = err }
    end

    local dryRun = payload.dryRun == true
    local desired = {}
    local created = 0
    local updated = 0
    local unchanged = 0
    local deleted = 0
    local changedPaths = {}
    local warnings = {}
    local sourceId = Ownership.sourceId(payload, "sync")

    for _, item in ipairs(payload.items) do
        local parts, pathErr = splitRelativePath(item.path)
        if not parts then
            return { ok = false, error = pathErr }
        end
        desired[table.concat(parts, "/")] = true
        local result, itemErr = upsertItem(root, item, dryRun, sourceId, payload.force == true)
        if not result then
            return { ok = false, error = itemErr }
        end
        if result.created then
            created += 1
            table.insert(changedPaths, result.path)
        elseif result.updated then
            updated += 1
            table.insert(changedPaths, result.path)
        else
            unchanged += 1
        end
    end

    if payload.delete == true then
        for path, instance in pairs(collectOwned(root)) do
            if not desired[path] then
                local canDelete, deleteErr = canMutateSynced(instance, sourceId, payload.force == true)
                if not canDelete then
                    return { ok = false, error = deleteErr }
                end
                deleted += 1
                table.insert(changedPaths, instance:GetFullName())
                if not dryRun then
                    instance:Destroy()
                end
            end
        end
    end

    return {
        ok = true,
        data = {
            parentPath = root:GetFullName(),
            created = created,
            updated = updated,
            deleted = deleted,
            unchanged = unchanged,
            dryRun = dryRun,
            changedPaths = changedPaths,
            warnings = warnings
        }
    }
end

return upsertFilesHandler
