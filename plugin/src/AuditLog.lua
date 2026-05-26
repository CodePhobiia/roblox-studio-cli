local HttpService = game:GetService("HttpService")
local ServerStorage = game:GetService("ServerStorage")
local Deserializer = require(script.Parent.Deserializer)

local AuditLog = {}

local MAX_RECORDS = 50
local FOLDER_NAME = "rsCommandHistory"

local function folder()
    local existing = ServerStorage:FindFirstChild(FOLDER_NAME)
    if existing and existing:IsA("Folder") then
        return existing
    end
    local created = Instance.new("Folder")
    created.Name = FOLDER_NAME
    created.Parent = ServerStorage
    return created
end

local function nowIso()
    return DateTime.now():ToIsoDate()
end

local function nowUnix()
    return DateTime.now().UnixTimestampMillis
end

local function decodeRecord(value)
    if not value or not value:IsA("StringValue") then
        return nil
    end
    local ok, decoded = pcall(function()
        return HttpService:JSONDecode(value.Value)
    end)
    if ok and type(decoded) == "table" then
        return decoded
    end
    return nil
end

local function encodeRecord(record)
    local ok, encoded = pcall(function()
        return HttpService:JSONEncode(record)
    end)
    if ok then
        return encoded
    end
    return "{}"
end

local function trim(logFolder)
    local values = {}
    for _, child in ipairs(logFolder:GetChildren()) do
        if child:IsA("StringValue") then
            table.insert(values, child)
        end
    end
    table.sort(values, function(a, b)
        return (a:GetAttribute("createdUnix") or 0) > (b:GetAttribute("createdUnix") or 0)
    end)
    for index = MAX_RECORDS + 1, #values do
        values[index]:Destroy()
    end
end

local function dataFrom(result)
    if type(result) == "table" and type(result.data) == "table" then
        return result.data
    end
    return {}
end

local function collectPaths(data)
    local paths = {}
    if type(data.changedPaths) == "table" then
        for _, path in ipairs(data.changedPaths) do
            table.insert(paths, tostring(path))
        end
    end
    for _, key in ipairs({ "rootPath", "path", "parentPath", "guiPath", "instancePath" }) do
        if type(data[key]) == "string" then
            table.insert(paths, data[key])
        end
    end
    return paths
end

function AuditLog.record(command, result)
    local data = dataFrom(result)
    local meta = type(command.payload) == "table" and command.payload._rs or {}
    local record = {
        commandId = tostring(command.commandId or HttpService:GenerateGUID(false)),
        kind = tostring(command.type or "unknown"),
        status = result.ok and "ok" or "failed",
        startedAt = nowIso(),
        cliVersion = type(meta) == "table" and meta.cliVersion or nil,
        protocolVersion = type(meta) == "table" and meta.protocolVersion or nil,
        pathsChanged = collectPaths(data),
        warnings = type(data.warnings) == "table" and data.warnings or {},
        error = result.ok and nil or tostring(result.error or "unknown error"),
        undoable = type(data.snapshotBefore) == "table" and type(data.restoreParentPath) == "string",
        snapshotBefore = data.snapshotBefore,
        restoreParentPath = data.restoreParentPath
    }
    local logFolder = folder()
    local value = Instance.new("StringValue")
    value.Name = record.commandId
    value.Value = encodeRecord(record)
    value:SetAttribute("createdUnix", nowUnix())
    value.Parent = logFolder
    trim(logFolder)
end

function AuditLog.list()
    local records = {}
    for _, child in ipairs(folder():GetChildren()) do
        local record = decodeRecord(child)
        if record then
            table.insert(records, record)
        end
    end
    table.sort(records, function(a, b)
        return tostring(a.startedAt or "") > tostring(b.startedAt or "")
    end)
    return records
end

function AuditLog.show(commandId)
    for _, record in ipairs(AuditLog.list()) do
        if record.commandId == commandId then
            return record
        end
    end
    return nil
end

local function rootNameFromBlob(blob)
    local rootSpec = type(blob) == "table" and type(blob.instances) == "table" and blob.instances[blob.root] or nil
    local props = type(rootSpec) == "table" and rootSpec.properties or nil
    if type(props) == "table" and type(props.Name) == "string" and props.Name ~= "" then
        return props.Name
    end
    return "Restored"
end

local function resolveParent(path)
    local StudioPath = require(script.Parent.StudioPath)
    return StudioPath.resolve(path)
end

function AuditLog.undo(commandId)
    local record = AuditLog.show(commandId)
    if not record then
        return nil, "history record not found: " .. tostring(commandId)
    end
    if type(record.snapshotBefore) ~= "table" or type(record.restoreParentPath) ~= "string" then
        return nil, "history record has no rollback snapshot"
    end
    local parent, err = resolveParent(record.restoreParentPath)
    if not parent then
        return nil, err
    end
    local rootName = rootNameFromBlob(record.snapshotBefore)
    local existing = parent:FindFirstChild(rootName)
    if existing then
        existing:Destroy()
    end
    local root, warnings = Deserializer.deserialize(record.snapshotBefore, parent)
    if not root then
        return nil, warnings
    end
    return {
        status = "restored",
        commandId = commandId,
        rootPath = root:GetFullName(),
        warnings = warnings or {}
    }, nil
end

return AuditLog
