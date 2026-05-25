local Importer = require(script.Parent.Parent.Importer)
local StudioPath = require(script.Parent.Parent.StudioPath)

local function importAssetHandler(payload)
    if type(payload.parentPath) ~= "string" then
        return { ok = false, error = "parentPath missing" }
    end
    if type(payload.name) ~= "string" then
        return { ok = false, error = "name missing" }
    end
    if type(payload.meshes) ~= "table" then
        return { ok = false, error = "meshes missing" }
    end

    local parent, err = StudioPath.resolve(payload.parentPath)
    if not parent then
        return { ok = false, error = err or ("parent path not found: " .. payload.parentPath) }
    end

    local root, dataOrErr = Importer.import(payload, parent)
    if not root then
        return { ok = false, error = dataOrErr }
    end
    return { ok = true, data = dataOrErr }
end

return importAssetHandler
