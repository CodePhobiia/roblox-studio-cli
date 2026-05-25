local ImportImage = require(script.Parent.Parent.ImportImage)
local Ownership = require(script.Parent.Parent.Ownership)
local StudioPath = require(script.Parent.Parent.StudioPath)

local function resolveContainer(parent, name)
    if parent:IsA("StarterGui") or parent:IsA("PlayerGui") then
        local guiName = StudioPath.sanitize(name, "ImportedGui")
        local existing = parent:FindFirstChild(guiName)
        if existing and existing:IsA("ScreenGui") then
            return existing, existing, nil
        end
        local screenGui = Instance.new("ScreenGui")
        screenGui.Name = guiName
        screenGui.ResetOnSpawn = false
        screenGui.Parent = parent
        return screenGui, screenGui, nil
    end
    if parent:IsA("LayerCollector") or parent:IsA("GuiObject") then
        return parent, parent, nil
    end
    return nil, nil, "parent must be StarterGui, PlayerGui, ScreenGui, SurfaceGui, BillboardGui, or a GuiObject"
end

local function applyScaleType(imageObject, scaleTypeName)
    if type(scaleTypeName) ~= "string" or scaleTypeName == "" then
        imageObject.ScaleType = Enum.ScaleType.Fit
        return nil
    end
    local value = Enum.ScaleType[scaleTypeName]
    if not value then
        return "unknown ScaleType: " .. tostring(scaleTypeName)
    end
    imageObject.ScaleType = value
    return nil
end

local function upsertImageObject(container, element)
    local kind = tostring(element.kind or "image")
    local className = kind == "button" and "ImageButton" or "ImageLabel"
    local name = StudioPath.sanitize(element.name, "Image")
    local existing = container:FindFirstChild(name)
    if existing and existing.ClassName ~= className then
        existing:Destroy()
        existing = nil
    end
    local imageObject = existing or ImportImage.createImageObject(kind)
    imageObject.Name = name
    imageObject.BackgroundTransparency = tonumber(element.backgroundTransparency) or 1
    imageObject.BorderSizePixel = 0
    imageObject.Size = UDim2.new(
        tonumber(element.sizeScaleX) or 0,
        tonumber(element.sizeOffsetX) or 0,
        tonumber(element.sizeScaleY) or 0,
        tonumber(element.sizeOffsetY) or 0
    )
    imageObject.Position = UDim2.new(
        tonumber(element.positionScaleX) or 0,
        tonumber(element.positionOffsetX) or 0,
        tonumber(element.positionScaleY) or 0,
        tonumber(element.positionOffsetY) or 0
    )
    imageObject.AnchorPoint = Vector2.new(tonumber(element.anchorX) or 0, tonumber(element.anchorY) or 0)
    if element.zIndex ~= nil then
        imageObject.ZIndex = math.floor(tonumber(element.zIndex) or imageObject.ZIndex)
    end
    local scaleErr = applyScaleType(imageObject, element.scaleType)
    if scaleErr then
        return nil, scaleErr
    end
    imageObject.Parent = container
    return imageObject, nil
end

local function importOne(container, element)
    local width = math.floor(tonumber(element.width) or 0)
    local height = math.floor(tonumber(element.height) or 0)
    if width < 1 or height < 1 then
        return nil, "element width and height must be positive"
    end
    local pixels, decodeErr, decodedBytes = ImportImage.decodeBase64(element.pixelsBase64)
    if not pixels then
        return nil, decodeErr
    end
    local expectedBytes = width * height * 4
    if decodedBytes ~= expectedBytes then
        return nil, "decoded PNG pixel byte count mismatch: expected " .. tostring(expectedBytes) .. ", got " .. tostring(decodedBytes)
    end

    local editableImage, imageErr = ImportImage.createEditableImage(width, height, pixels)
    if not editableImage then
        return nil, imageErr
    end

    local imageObject, objectErr = upsertImageObject(container, element)
    if not imageObject then
        editableImage:Destroy()
        return nil, objectErr
    end

    local okContent, contentErr = pcall(function()
        imageObject.ImageContent = Content.fromObject(editableImage)
    end)
    if not okContent then
        editableImage:Destroy()
        return nil, "ImageContent assignment failed: " .. tostring(contentErr)
    end
    return imageObject, nil
end

local function importUiPackHandler(payload)
    if type(payload.parentPath) ~= "string" then
        return { ok = false, error = "parentPath missing" }
    end
    if type(payload.name) ~= "string" then
        return { ok = false, error = "name missing" }
    end
    if type(payload.elements) ~= "table" then
        return { ok = false, error = "elements missing" }
    end

    local parent, resolveErr = StudioPath.resolve(payload.parentPath)
    if not parent then
        return { ok = false, error = resolveErr }
    end

    local container, guiRoot, containerErr = resolveContainer(parent, payload.name)
    if not container then
        return { ok = false, error = containerErr }
    end
    local sourceId = Ownership.sourceId(payload, "ui")
    Ownership.stamp(guiRoot, sourceId)

    local elementPaths = {}
    local warnings = {}
    for index, element in ipairs(payload.elements) do
        local imageObject, err = importOne(container, element)
        if not imageObject then
            return { ok = false, error = "element " .. tostring(index) .. ": " .. tostring(err) }
        end
        Ownership.stamp(imageObject, sourceId)
        table.insert(elementPaths, imageObject:GetFullName())
    end

    return {
        ok = true,
        data = {
            guiPath = guiRoot:GetFullName(),
            elementCount = #elementPaths,
            elementPaths = elementPaths,
            warnings = warnings
        }
    }
end

return importUiPackHandler
