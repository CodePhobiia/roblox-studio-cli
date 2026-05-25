local AssetService = game:GetService("AssetService")
local Ownership = require(script.Parent.Ownership)

local ImportImage = {}

local BASE64 = {}
do
    local chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
    for i = 1, #chars do
        BASE64[string.byte(chars, i)] = i - 1
    end
end

local function sanitize(value)
    value = tostring(value or "ImportedImage")
    value = string.gsub(value, "[<>:\"/\\|%?%*%c]", "_")
    value = string.gsub(value, "^%s+", "")
    value = string.gsub(value, "%s+$", "")
    if value == "" then
        value = "ImportedImage"
    end
    if #value > 80 then
        value = string.sub(value, 1, 80)
    end
    return value
end

local function decodeBase64(value)
    if type(value) ~= "string" then
        return nil, "pixelsBase64 must be a string"
    end

    local meaningful = 0
    for i = 1, #value do
        local byte = string.byte(value, i)
        if BASE64[byte] then
            meaningful += 1
        elseif byte == 61 then
            break
        elseif byte ~= 9 and byte ~= 10 and byte ~= 13 and byte ~= 32 then
            return nil, "pixelsBase64 contains an invalid base64 character"
        end
    end

    local out = buffer.create(math.floor(meaningful * 6 / 8))
    local acc = 0
    local bits = 0
    local offset = 0

    for i = 1, #value do
        local byte = string.byte(value, i)
        if byte == 61 then
            break
        end
        local decoded = BASE64[byte]
        if decoded then
            acc = acc * 64 + decoded
            bits += 6
            while bits >= 8 do
                bits -= 8
                local divisor = 2 ^ bits
                local outByte = math.floor(acc / divisor) % 256
                buffer.writeu8(out, offset, outByte)
                offset += 1
                acc = acc % divisor
            end
        elseif byte ~= 9 and byte ~= 10 and byte ~= 13 and byte ~= 32 then
            return nil, "pixelsBase64 contains an invalid base64 character"
        end
    end

    return out, nil, offset
end

local function createEditableImage(width, height, pixels)
    local okImage, editableImageOrErr = pcall(function()
        return AssetService:CreateEditableImage({
            Size = Vector2.new(width, height)
        })
    end)
    if not okImage or not editableImageOrErr then
        return nil, "CreateEditableImage failed: " .. tostring(editableImageOrErr)
    end

    local editableImage = editableImageOrErr
    local okWrite, writeErr = pcall(function()
        editableImage:WritePixelsBuffer(Vector2.zero, Vector2.new(width, height), pixels)
    end)
    if not okWrite then
        pcall(function()
            editableImage:Destroy()
        end)
        return nil, "WritePixelsBuffer failed: " .. tostring(writeErr)
    end
    return editableImage
end

local function shouldCreateScreenGui(parent)
    return parent:IsA("StarterGui") or parent:IsA("PlayerGui")
end

local function resolveContainer(parent, name)
    if shouldCreateScreenGui(parent) then
        local screenGui = Instance.new("ScreenGui")
        screenGui.Name = name .. "Gui"
        screenGui.ResetOnSpawn = false
        screenGui.Parent = parent
        return screenGui, screenGui
    end
    if parent:IsA("LayerCollector") or parent:IsA("GuiObject") then
        return parent, parent
    end
    return nil, nil, "parent must be StarterGui, PlayerGui, ScreenGui, SurfaceGui, BillboardGui, or a GuiObject"
end

local function createImageObject(kind)
    if kind == "button" then
        local button = Instance.new("ImageButton")
        button.AutoButtonColor = true
        button.Modal = false
        return button
    end
    return Instance.new("ImageLabel")
end

function ImportImage.import(payload, parent)
    if type(payload) ~= "table" then
        return nil, "payload must be a table"
    end

    local width = tonumber(payload.width)
    local height = tonumber(payload.height)
    local uiWidth = tonumber(payload.uiWidth)
    local uiHeight = tonumber(payload.uiHeight)
    if not width or not height or width < 1 or height < 1 then
        return nil, "width and height must be positive numbers"
    end
    if not uiWidth or not uiHeight or uiWidth < 1 or uiHeight < 1 then
        return nil, "uiWidth and uiHeight must be positive numbers"
    end
    width = math.floor(width)
    height = math.floor(height)
    uiWidth = math.floor(uiWidth)
    uiHeight = math.floor(uiHeight)

    local expectedBytes = width * height * 4
    local pixels, decodeErr, decodedBytes = decodeBase64(payload.pixelsBase64)
    if not pixels then
        return nil, decodeErr
    end
    if decodedBytes ~= expectedBytes then
        return nil, "decoded PNG pixel byte count mismatch: expected " .. tostring(expectedBytes) .. ", got " .. tostring(decodedBytes)
    end

    local name = sanitize(payload.name)
    local sourceId = Ownership.sourceId(payload, "image")
    local container, guiRoot, containerErr = resolveContainer(parent, name)
    if not container then
        return nil, containerErr
    end

    local editableImage, imageErr = createEditableImage(width, height, pixels)
    if not editableImage then
        if guiRoot ~= parent then
            guiRoot:Destroy()
        end
        return nil, imageErr
    end

    local kind = tostring(payload.kind or "image")
    local imageObject = createImageObject(kind)
    imageObject.Name = name
    imageObject.BackgroundTransparency = 1
    imageObject.BorderSizePixel = 0
    imageObject.Size = UDim2.fromOffset(uiWidth, uiHeight)
    imageObject.Position = UDim2.fromOffset(tonumber(payload.positionX) or 0, tonumber(payload.positionY) or 0)
    imageObject.ScaleType = Enum.ScaleType.Fit

    local okContent, contentErr = pcall(function()
        imageObject.ImageContent = Content.fromObject(editableImage)
    end)
    if not okContent then
        imageObject:Destroy()
        pcall(function()
            editableImage:Destroy()
        end)
        if guiRoot ~= parent then
            guiRoot:Destroy()
        end
        return nil, "ImageContent assignment failed: " .. tostring(contentErr)
    end

    imageObject.Parent = container
    Ownership.stamp(guiRoot, sourceId)
    Ownership.stamp(imageObject, sourceId)

    return imageObject, {
        rootPath = guiRoot:GetFullName(),
        guiPath = guiRoot:GetFullName(),
        imagePath = imageObject:GetFullName(),
        className = imageObject.ClassName,
        width = width,
        height = height,
        warnings = {}
    }
end

ImportImage.sanitize = sanitize
ImportImage.decodeBase64 = decodeBase64
ImportImage.createEditableImage = createEditableImage
ImportImage.createImageObject = createImageObject

return ImportImage
