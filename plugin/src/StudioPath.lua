local StudioPath = {}

local function splitPath(path)
    local parts = {}
    for part in string.gmatch(path, "[^%.]+") do
        table.insert(parts, part)
    end
    return parts
end

local function findUniqueChild(parent, name)
    local found = nil
    local count = 0
    for _, child in ipairs(parent:GetChildren()) do
        if child.Name == name then
            found = child
            count += 1
        end
    end
    if count > 1 then
        return nil, "ambiguous path segment '" .. tostring(name) .. "' under " .. parent:GetFullName()
    end
    return found, nil
end

function StudioPath.resolve(path)
    if type(path) ~= "string" or path == "" then
        return nil, "path missing"
    end
    if path == "game" or path == "DataModel" then
        return game, nil
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
            local err = nil
            node, err = findUniqueChild(node, part)
            if err then
                return nil, err
            end
        end
        if not node then
            return nil, "path not found: " .. path
        end
    end
    return node, nil
end

function StudioPath.sanitize(value, fallback)
    value = tostring(value or fallback or "Instance")
    value = string.gsub(value, "[<>:\"/\\|%?%*%c]", "_")
    value = string.gsub(value, "^%s+", "")
    value = string.gsub(value, "%s+$", "")
    if value == "" then
        value = fallback or "Instance"
    end
    if #value > 80 then
        value = string.sub(value, 1, 80)
    end
    return value
end

return StudioPath
