local StudioPath = require(script.Parent.Parent.StudioPath)

local function decodeValue(value)
    if type(value) ~= "table" then
        return value, nil
    end

    local valueType = value.type
    if valueType == "Vector3" then
        return Vector3.new(tonumber(value.x) or 0, tonumber(value.y) or 0, tonumber(value.z) or 0), nil
    elseif valueType == "Color3" then
        return Color3.new(tonumber(value.r) or 0, tonumber(value.g) or 0, tonumber(value.b) or 0), nil
    elseif valueType == "UDim2" then
        return UDim2.new(
            tonumber(value.xScale) or 0,
            tonumber(value.xOffset) or 0,
            tonumber(value.yScale) or 0,
            tonumber(value.yOffset) or 0
        ), nil
    elseif valueType == "Enum" then
        local enumType = Enum[tostring(value.enumType or "")]
        if not enumType then
            return nil, "unknown enum type: " .. tostring(value.enumType)
        end
        local enumValue = enumType[tostring(value.enumItem or "")]
        if not enumValue then
            return nil, "unknown enum item: " .. tostring(value.enumItem)
        end
        return enumValue, nil
    end

    return value, nil
end

local function createHandler(payload)
    if type(payload.parentPath) ~= "string" then
        return { ok = false, error = "parentPath missing" }
    end
    if type(payload.className) ~= "string" then
        return { ok = false, error = "className missing" }
    end
    if type(payload.name) ~= "string" then
        return { ok = false, error = "name missing" }
    end

    local parent, err = StudioPath.resolve(payload.parentPath)
    if not parent then
        return { ok = false, error = err }
    end

    local okCreate, instanceOrErr = pcall(function()
        return Instance.new(payload.className)
    end)
    if not okCreate or not instanceOrErr then
        return { ok = false, error = "could not create class '" .. tostring(payload.className) .. "': " .. tostring(instanceOrErr) }
    end

    local instance = instanceOrErr
    local warnings = {}
    local okName, nameErr = pcall(function()
        instance.Name = payload.name
    end)
    if not okName then
        instance:Destroy()
        return { ok = false, error = "could not set Name: " .. tostring(nameErr) }
    end

    for _, property in ipairs(payload.properties or {}) do
        local propertyName = property.name
        if type(propertyName) ~= "string" or propertyName == "" then
            instance:Destroy()
            return { ok = false, error = "property name missing" }
        end
        local decoded, decodeErr = decodeValue(property.value)
        if decodeErr then
            instance:Destroy()
            return { ok = false, error = propertyName .. ": " .. decodeErr }
        end
        local okSet, setErr = pcall(function()
            instance[propertyName] = decoded
        end)
        if not okSet then
            instance:Destroy()
            return { ok = false, error = propertyName .. ": " .. tostring(setErr) }
        end
    end

    instance.Parent = parent
    return {
        ok = true,
        data = {
            path = instance:GetFullName(),
            className = instance.ClassName,
            warnings = warnings
        }
    }
end

return createHandler
