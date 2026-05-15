local Exporter = require(script.Parent.Parent.Exporter)

local function splitPath(path)
    local parts = {}
    for part in string.gmatch(path, "[^%.]+") do
        table.insert(parts, part)
    end
    return parts
end

local function resolvePath(path)
    if path == "game" or path == "DataModel" then
        return game
    end

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

local function exportHandler(payload)
    if type(payload.path) ~= "string" then
        return { ok = false, error = "path missing" }
    end

    local target = resolvePath(payload.path)
    if not target then
        return { ok = false, error = "path not found: " .. payload.path }
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
