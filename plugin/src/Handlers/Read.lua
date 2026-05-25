local Serializer = require(script.Parent.Parent.Serializer)
local StudioPath = require(script.Parent.Parent.StudioPath)

local function readHandler(payload)
    if type(payload.path) ~= "string" then
        return { ok = false, error = "path missing" }
    end
    local target, err = StudioPath.resolve(payload.path)
    if not target then
        return { ok = false, error = err or ("path not found: " .. payload.path) }
    end
    return { ok = true, data = Serializer.readInstance(target, tonumber(payload.depth) or 3) }
end

return readHandler
