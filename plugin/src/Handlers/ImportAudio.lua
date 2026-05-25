local StudioPath = require(script.Parent.Parent.StudioPath)
local Ownership = require(script.Parent.Parent.Ownership)

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

local function importAudioHandler(payload)
    if type(payload.parentPath) ~= "string" then
        return { ok = false, error = "parentPath missing" }
    end
    if type(payload.sounds) ~= "table" then
        return { ok = false, error = "sounds missing" }
    end

    local parent, err = StudioPath.resolve(payload.parentPath)
    if not parent then
        return { ok = false, error = err }
    end

    local soundPaths = {}
    local warnings = {}
    local sourceId = Ownership.sourceId(payload, "audio")
    for index, spec in ipairs(payload.sounds) do
        local soundId = normalizeAssetId(spec.assetId)
        if not soundId then
            return { ok = false, error = "sound " .. tostring(index) .. " missing assetId" }
        end
        local name = StudioPath.sanitize(spec.name, "Sound")
        local existing = parent:FindFirstChild(name)
        if existing and not existing:IsA("Sound") then
            existing:Destroy()
            existing = nil
        end
        local sound = existing or Instance.new("Sound")
        sound.Name = name
        sound.SoundId = soundId
        if spec.volume ~= nil then
            sound.Volume = tonumber(spec.volume) or sound.Volume
        end
        if spec.playbackSpeed ~= nil then
            sound.PlaybackSpeed = tonumber(spec.playbackSpeed) or sound.PlaybackSpeed
        end
        sound.Looped = spec.looped == true
        sound.Parent = parent
        Ownership.stamp(sound, sourceId)
        table.insert(soundPaths, sound:GetFullName())
    end

    return {
        ok = true,
        data = {
            parentPath = parent:GetFullName(),
            soundCount = #soundPaths,
            soundPaths = soundPaths,
            warnings = warnings
        }
    }
end

return importAudioHandler
