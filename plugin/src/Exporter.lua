local HttpService = game:GetService("HttpService")
local Allowlist = require(script.Parent.PropertyAllowlist)
local Serializer = require(script.Parent.Serializer)

local Exporter = {}

local SCRIPT_EXTENSIONS = {
    Script = ".server.lua",
    LocalScript = ".client.lua",
    ModuleScript = ".module.lua"
}

local function sanitize(value)
    value = tostring(value or "unnamed")
    value = string.gsub(value, "[<>:\"/\\|%?%*%c]", "_")
    value = string.gsub(value, "^%s+", "")
    value = string.gsub(value, "%s+$", "")
    if value == "" then
        value = "unnamed"
    end
    if #value > 80 then
        value = string.sub(value, 1, 80)
    end
    return value
end

local function joinPath(a, b)
    if a == "" then
        return b
    end
    return a .. "/" .. b
end

local function padded(index)
    return string.format("%04d", index)
end

local function cloneJson(value)
    local ok, encoded = pcall(function()
        return HttpService:JSONEncode(value)
    end)
    if not ok then
        return value
    end
    local okDecode, decoded = pcall(function()
        return HttpService:JSONDecode(encoded)
    end)
    return okDecode and decoded or value
end

local function addFile(files, path, kind, content, json)
    table.insert(files, {
        path = path,
        kind = kind,
        content = content,
        json = json
    })
end

local function addAssetFiles(files, dir, metadata)
    for property, value in pairs(metadata.properties or {}) do
        local kind = Allowlist.contentKindForClass(metadata.className, property)
        if kind and type(value) == "string" and value ~= "" then
            addFile(files, joinPath(dir, "assets/" .. sanitize(property) .. ".asset.json"), kind, nil, {
                instancePath = metadata.fullPath,
                className = metadata.className,
                property = property,
                assetUri = value,
                assetKind = kind
            })
        end
    end
end

local function addScriptFile(files, dir, instance, metadata)
    local extension = SCRIPT_EXTENSIONS[instance.ClassName]
    if not extension then
        return
    end

    local ok, source = pcall(function()
        return instance.Source
    end)
    if ok and type(source) == "string" then
        metadata.properties = metadata.properties or {}
        metadata.properties.Source = nil
        addFile(files, joinPath(dir, sanitize(instance.Name) .. extension), "script", source, nil)
    end
end

function Exporter.export(root, maxDepth)
    local files = {}
    local warnings = {}
    local nextIndex = 0

    local function walk(instance, parentDir, remainingDepth)
        local index = nextIndex
        nextIndex += 1
        local dirName = padded(index) .. "_" .. sanitize(instance.Name) .. "_" .. sanitize(instance.ClassName)
        local dir = joinPath(parentDir, dirName)
        local metadata = Serializer.readInstance(instance, 0)
        metadata.childrenCount = #instance:GetChildren()
        metadata.exportDirectory = dir

        if metadata.warnings then
            for _, warning in ipairs(metadata.warnings) do
                table.insert(warnings, warning)
            end
            metadata.warnings = nil
        end

        metadata = cloneJson(metadata)
        addScriptFile(files, dir, instance, metadata)
        addAssetFiles(files, dir, metadata)
        addFile(files, joinPath(dir, "instance.json"), "metadata", nil, metadata)

        if remainingDepth == nil or remainingDepth > 0 then
            local childDepth = nil
            if remainingDepth ~= nil then
                childDepth = remainingDepth - 1
            end
            for _, child in ipairs(instance:GetChildren()) do
                walk(child, dir, childDepth)
            end
        end
    end

    walk(root, sanitize(root.Name), maxDepth)
    addFile(files, sanitize(root.Name) .. "/export_manifest.json", "manifest", nil, {
        rootPath = root:GetFullName(),
        fileCount = #files + 1,
        warnings = warnings
    })

    return {
        rootPath = root:GetFullName(),
        files = files,
        warnings = warnings
    }
end

return Exporter
