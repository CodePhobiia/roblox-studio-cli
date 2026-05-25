local StudioPath = require(script.Parent.Parent.StudioPath)
local Inspector = require(script.Parent.Parent.Inspector)

local function snapshotHandler(payload)
    if type(payload.path) ~= "string" then
        return { ok = false, error = "path missing" }
    end
    local target, err = StudioPath.resolve(payload.path)
    if not target then
        return { ok = false, error = err }
    end
    return { ok = true, data = Inspector.snapshot(target, payload.includePaths == true) }
end

return snapshotHandler
