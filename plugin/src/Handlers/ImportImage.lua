local ImportImage = require(script.Parent.Parent.ImportImage)
local StudioPath = require(script.Parent.Parent.StudioPath)

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

    local parent, err = StudioPath.resolve(payload.parentPath)
    if not parent then
        return { ok = false, error = err or ("parent path not found: " .. payload.parentPath) }
    end

    local imageObject, dataOrErr = ImportImage.import(payload, parent)
    if not imageObject then
        return { ok = false, error = dataOrErr }
    end
    return { ok = true, data = dataOrErr }
end

return importImageHandler
