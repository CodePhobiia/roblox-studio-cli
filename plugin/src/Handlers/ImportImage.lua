local ImportImage = require(script.Parent.Parent.ImportImage)

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

local function importImageHandler(payload)
    if type(payload.parentPath) ~= "string" then
        return { ok = false, error = "parentPath missing" }
    end
    if type(payload.name) ~= "string" then
        return { ok = false, error = "name missing" }
    end
    if type(payload.kind) ~= "string" then
        return { ok = false, error = "kind missing" }
    end

    local parent = resolvePath(payload.parentPath)
    if not parent then
        return { ok = false, error = "parent path not found: " .. payload.parentPath }
    end

    local imageObject, dataOrErr = ImportImage.import(payload, parent)
    if not imageObject then
        return { ok = false, error = dataOrErr }
    end
    return { ok = true, data = dataOrErr }
end

return importImageHandler
