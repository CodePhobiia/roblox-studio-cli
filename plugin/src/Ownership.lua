local HttpService = game:GetService("HttpService")

local Ownership = {}

local function importedAt()
    return DateTime.now():ToIsoDate()
end

function Ownership.sourceId(payload, fallbackPrefix)
    if type(payload) == "table" and type(payload.sourceId) == "string" and payload.sourceId ~= "" then
        return payload.sourceId
    end
    return tostring(fallbackPrefix or "rs") .. "-" .. HttpService:GenerateGUID(false)
end

function Ownership.stamp(instance, sourceId, packageId)
    if not instance then
        return
    end
    pcall(function()
        if type(sourceId) == "string" and sourceId ~= "" then
            instance:SetAttribute("rsSourceId", sourceId)
        end
        if type(packageId) == "string" and packageId ~= "" then
            instance:SetAttribute("rsPackageId", packageId)
        end
        instance:SetAttribute("rsImportedAt", importedAt())
        instance:SetAttribute("rsManagedBy", "rs")
    end)
end

function Ownership.isOwned(instance, sourceId, packageId)
    if not instance then
        return false
    end
    local managedBy = nil
    local existingSourceId = nil
    local existingPackageId = nil
    pcall(function()
        managedBy = instance:GetAttribute("rsManagedBy")
        existingSourceId = instance:GetAttribute("rsSourceId")
        existingPackageId = instance:GetAttribute("rsPackageId")
    end)
    if managedBy == "rs" then
        return true
    end
    if type(sourceId) == "string" and sourceId ~= "" and existingSourceId == sourceId then
        return true
    end
    if type(packageId) == "string" and packageId ~= "" and existingPackageId == packageId then
        return true
    end
    return false
end

function Ownership.canMutate(instance, options)
    options = options or {}
    if options.force == true then
        return true, nil
    end
    if Ownership.isOwned(instance, options.sourceId, options.packageId) then
        return true, nil
    end
    local path = "<unknown>"
    pcall(function()
        path = instance:GetFullName()
    end)
    return false, "refusing to overwrite user-owned/manual instance: " .. path .. " (pass --force to override)"
end

function Ownership.stableSourceId(packageId, localId, fallback)
    if type(packageId) == "string" and packageId ~= "" and type(localId) == "string" and localId ~= "" then
        return packageId .. ":" .. localId
    end
    return fallback
end

return Ownership
