local Serializer = require(script.Parent.Parent.Serializer)

local function splitPath(path)
    local parts = {}
    for part in string.gmatch(path, "[^%.]+") do
        table.insert(parts, part)
    end
    return parts
end

local function resolvePath(path)
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

local function serializeHandler(payload)
    if type(payload.path) ~= "string" then
        return { ok = false, error = "path missing" }
    end
    local target = resolvePath(payload.path)
    if not target then
        return { ok = false, error = "path not found: " .. payload.path }
    end
    return { ok = true, data = Serializer.serialize(target) }
end

return serializeHandler
