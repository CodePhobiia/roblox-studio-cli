local StudioPath = require(script.Parent.Parent.StudioPath)
local Ownership = require(script.Parent.Parent.Ownership)

local SCRIPT_CLASSES = {
    Script = true,
    LocalScript = true,
    ModuleScript = true
}

local function splitRelativePath(path)
    local parts = {}
    for part in string.gmatch(tostring(path or ""), "[^/\\]+") do
        if part ~= "." and part ~= "" then
            table.insert(parts, part)
        end
    end
    return parts
end

local function ensureFolder(parent, name, dryRun, sourceId)
    local existing = parent:FindFirstChild(name)
    if existing then
        if not existing:IsA("Folder") then
            return nil, "path segment exists but is not a Folder: " .. existing:GetFullName()
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

local function ensureParent(root, relativePath, dryRun, sourceId)
    local parts = splitRelativePath(relativePath)
    local parent = root
    for i = 1, math.max(#parts - 1, 0) do
        local nextParent, err = ensureFolder(parent, parts[i], dryRun, sourceId)
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
    local parent, err = ensureParent(root, item.path, dryRun, sourceId)
    if not parent then
        return nil, err
    end

    local existing = parent:FindFirstChild(name)
    local created = false
    local updated = false
    if existing then
        local canMutate, mutateErr = Ownership.canMutate(existing, {
            sourceId = sourceId,
            force = force
        })
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
        instance:SetAttribute("rsSyncPath", tostring(item.path or ""))
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
        desired[tostring(item.path or "")] = true
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
