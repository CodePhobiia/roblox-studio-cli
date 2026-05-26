local Ownership = require(script.Parent.Parent.Ownership)
local StudioPath = require(script.Parent.Parent.StudioPath)

local function normalizeAssetId(value)
    if type(value) ~= "string" or value == "" then
        return nil
    end
    if string.match(value, "^rbxassetid://%d+$") then
        return value
    end
    if string.match(value, "^%d+$") then
        return "rbxassetid://" .. value
    end
    return value
end

local function directChildWithSourceId(parent, sourceId)
    if type(sourceId) ~= "string" or sourceId == "" then
        return nil
    end
    for _, child in ipairs(parent:GetChildren()) do
        local ok, childSourceId, managedBy = pcall(function()
            return child:GetAttribute("rsSourceId"), child:GetAttribute("rsManagedBy")
        end)
        if ok and childSourceId == sourceId and managedBy == "rs" then
            return child
        end
    end
    return nil
end

local function ensureImageContainer(parent, name, sourceId)
    if parent:IsA("StarterGui") or parent:IsA("PlayerGui") then
        local existing = directChildWithSourceId(parent, sourceId)
        if existing and existing:IsA("ScreenGui") then
            existing.Name = name .. "Gui"
            existing.ResetOnSpawn = false
            return existing, existing
        elseif existing then
            existing:Destroy()
        end
        local gui = Instance.new("ScreenGui")
        gui.Name = name .. "Gui"
        gui.ResetOnSpawn = false
        gui.Parent = parent
        return gui, gui
    end
    return parent, parent
end

local function importImage(payload, parent, assetId, sourceId)
    local name = StudioPath.sanitize(payload.name, "UploadedImage")
    local container, root = ensureImageContainer(parent, name, sourceId)
    local uiKind = tostring(payload.uiKind or "image")
    local className = uiKind == "button" and "ImageButton" or "ImageLabel"
    local object = directChildWithSourceId(container, sourceId)
    if object and object.ClassName ~= className then
        object:Destroy()
        object = nil
    end
    if not object then
        object = Instance.new(className)
    end
    object.Name = name
    object.BackgroundTransparency = 1
    object.BorderSizePixel = 0
    object.Size = UDim2.fromOffset(tonumber(payload.uiWidth) or 128, tonumber(payload.uiHeight) or 128)
    object.Position = UDim2.fromOffset(tonumber(payload.positionX) or 0, tonumber(payload.positionY) or 0)
    object.ScaleType = Enum.ScaleType.Fit
    object.Image = assetId
    object.Parent = container
    Ownership.stamp(root, sourceId)
    Ownership.stamp(object, sourceId)
    return root, object, {}
end

local function importAudio(payload, parent, assetId, sourceId)
    local sound = directChildWithSourceId(parent, sourceId)
    if sound and not sound:IsA("Sound") then
        sound:Destroy()
        sound = nil
    end
    sound = sound or Instance.new("Sound")
    sound.Name = StudioPath.sanitize(payload.name, "UploadedSound")
    sound.SoundId = assetId
    if payload.volume ~= nil then
        sound.Volume = tonumber(payload.volume) or sound.Volume
    end
    if payload.playbackSpeed ~= nil then
        sound.PlaybackSpeed = tonumber(payload.playbackSpeed) or sound.PlaybackSpeed
    end
    sound.Looped = payload.looped == true
    sound.Parent = parent
    Ownership.stamp(sound, sourceId)
    return sound, sound, {}
end

local function importUploadedHandler(payload)
    if type(payload.parentPath) ~= "string" then
        return { ok = false, error = "parentPath missing" }
    end
    if type(payload.kind) ~= "string" then
        return { ok = false, error = "kind missing" }
    end
    local assetId = normalizeAssetId(payload.assetId)
    if not assetId then
        return { ok = false, error = "assetId missing" }
    end
    local parent, err = StudioPath.resolve(payload.parentPath)
    if not parent then
        return { ok = false, error = err }
    end

    local sourceId = Ownership.sourceId(payload, "uploaded")
    local root, instance, warnings
    if payload.kind == "image" then
        root, instance, warnings = importImage(payload, parent, assetId, sourceId)
    elseif payload.kind == "audio" then
        root, instance, warnings = importAudio(payload, parent, assetId, sourceId)
    else
        return { ok = false, error = "unsupported uploaded asset kind: " .. tostring(payload.kind) }
    end

    return {
        ok = true,
        data = {
            rootPath = root:GetFullName(),
            instancePath = instance:GetFullName(),
            className = instance.ClassName,
            assetId = assetId,
            warnings = warnings
        }
    }
end

return importUploadedHandler
