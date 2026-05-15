local Encoders = {}

local function array(tag, ...)
    local result = { tag }
    for _, value in ipairs({ ... }) do
        table.insert(result, value)
    end
    return result
end

function Encoders.encode(value, refs, seen)
    local valueType = typeof(value)

    if value == nil or valueType == "string" or valueType == "number" or valueType == "boolean" then
        return value
    elseif valueType == "table" then
        seen = seen or {}
        if seen[value] then
            return nil, "cyclic table"
        end
        seen[value] = true

        local result = {}
        for key, child in pairs(value) do
            if type(key) == "string" or type(key) == "number" then
                local encoded, warning = Encoders.encode(child, refs, seen)
                if encoded ~= nil then
                    result[key] = encoded
                elseif warning and child ~= nil then
                    seen[value] = nil
                    return nil, warning
                end
            end
        end

        seen[value] = nil
        return result
    elseif valueType == "Vector3" then
        return array("Vector3", value.X, value.Y, value.Z)
    elseif valueType == "Vector2" then
        return array("Vector2", value.X, value.Y)
    elseif valueType == "CFrame" then
        return array("CFrame", value:GetComponents())
    elseif valueType == "Color3" then
        return array("Color3", value.R, value.G, value.B)
    elseif valueType == "BrickColor" then
        return array("BrickColor", value.Number)
    elseif valueType == "UDim" then
        return array("UDim", value.Scale, value.Offset)
    elseif valueType == "UDim2" then
        return array("UDim2", value.X.Scale, value.X.Offset, value.Y.Scale, value.Y.Offset)
    elseif valueType == "Rect" then
        return array("Rect", value.Min.X, value.Min.Y, value.Max.X, value.Max.Y)
    elseif valueType == "EnumItem" then
        local enumTypeName, enumItemName = string.match(tostring(value), "^Enum%.([^.]+)%.(.+)$")
        if enumTypeName and enumItemName then
            return array("Enum", enumTypeName, enumItemName)
        end
        return nil, "unsupported enum item: " .. tostring(value)
    elseif valueType == "ColorSequence" then
        local keypoints = {}
        for _, keypoint in ipairs(value.Keypoints) do
            table.insert(keypoints, { t = keypoint.Time, c = { keypoint.Value.R, keypoint.Value.G, keypoint.Value.B } })
        end
        return { "ColorSequence", keypoints }
    elseif valueType == "NumberSequence" then
        local keypoints = {}
        for _, keypoint in ipairs(value.Keypoints) do
            table.insert(keypoints, { t = keypoint.Time, v = keypoint.Value, e = keypoint.Envelope })
        end
        return { "NumberSequence", keypoints }
    elseif valueType == "NumberRange" then
        return array("NumberRange", value.Min, value.Max)
    elseif valueType == "PhysicalProperties" then
        return array("PhysicalProperties", value.Density, value.Friction, value.Elasticity, value.FrictionWeight, value.ElasticityWeight)
    elseif valueType == "Instance" then
        if refs and refs[value] then
            return { "Ref", refs[value] }
        end
        if not refs then
            return { "InstancePath", value:GetFullName() }
        end
        return nil, "external instance reference: " .. value:GetFullName()
    end

    return nil, "unsupported property value type: " .. tostring(valueType)
end

function Encoders.decode(value, refs)
    if type(value) ~= "table" then
        return value
    end

    local tag = value[1]
    if tag == "Vector3" then
        return Vector3.new(value[2], value[3], value[4])
    elseif tag == "Vector2" then
        return Vector2.new(value[2], value[3])
    elseif tag == "CFrame" then
        return CFrame.new(table.unpack(value, 2, 13))
    elseif tag == "Color3" then
        return Color3.new(value[2], value[3], value[4])
    elseif tag == "BrickColor" then
        return BrickColor.new(value[2])
    elseif tag == "UDim" then
        return UDim.new(value[2], value[3])
    elseif tag == "UDim2" then
        return UDim2.new(value[2], value[3], value[4], value[5])
    elseif tag == "Rect" then
        return Rect.new(value[2], value[3], value[4], value[5])
    elseif tag == "Enum" then
        local enumType = Enum[value[2]]
        if enumType then
            return enumType[value[3]]
        end
        return nil, "unknown enum type: " .. tostring(value[2])
    elseif tag == "ColorSequence" then
        local keypoints = {}
        for _, item in ipairs(value[2] or {}) do
            local c = item.c or { 1, 1, 1 }
            table.insert(keypoints, ColorSequenceKeypoint.new(item.t, Color3.new(c[1], c[2], c[3])))
        end
        return ColorSequence.new(keypoints)
    elseif tag == "NumberSequence" then
        local keypoints = {}
        for _, item in ipairs(value[2] or {}) do
            table.insert(keypoints, NumberSequenceKeypoint.new(item.t, item.v, item.e or 0))
        end
        return NumberSequence.new(keypoints)
    elseif tag == "NumberRange" then
        return NumberRange.new(value[2], value[3])
    elseif tag == "PhysicalProperties" then
        return PhysicalProperties.new(value[2], value[3], value[4], value[5], value[6])
    elseif tag == "Ref" then
        local ref = refs and refs[value[2]]
        if ref then
            return ref
        end
        return nil, "missing instance reference: " .. tostring(value[2])
    elseif tag == "InstancePath" then
        return nil, "cannot deserialize external instance path: " .. tostring(value[2])
    end

    return value
end

return Encoders
