local Exporter = require(script.Parent.Parent.Exporter)
local StudioPath = require(script.Parent.Parent.StudioPath)

local function exportHandler(payload)
    if type(payload.path) ~= "string" then
        return { ok = false, error = "path missing" }
    end

    local target, err = StudioPath.resolve(payload.path)
    if not target then
        return { ok = false, error = err or ("path not found: " .. payload.path) }
    end

    local depth = nil
    if payload.depth ~= nil then
        depth = tonumber(payload.depth)
        if not depth or depth < 0 then
            return { ok = false, error = "depth must be a non-negative number" }
        end
    end

    return { ok = true, data = Exporter.export(target, depth) }
end

return exportHandler
