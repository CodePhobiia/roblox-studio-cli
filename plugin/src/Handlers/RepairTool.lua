local StudioPath = require(script.Parent.Parent.StudioPath)
local Inspector = require(script.Parent.Parent.Inspector)

local function repairToolHandler(payload)
    if type(payload.path) ~= "string" then
        return { ok = false, error = "path missing" }
    end
    local target, err = StudioPath.resolve(payload.path)
    if not target then
        return { ok = false, error = err }
    end
    if not target:IsA("Tool") then
        return { ok = false, error = "path must resolve to a Tool: " .. payload.path }
    end

    local result, repairErr = Inspector.repairTool(target, payload)
    if not result then
        return { ok = false, error = repairErr }
    end
    return { ok = true, data = result }
end

return repairToolHandler
