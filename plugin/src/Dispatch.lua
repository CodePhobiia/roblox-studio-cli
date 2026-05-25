local Dispatch = {}
local handlers = {}
local AuditLog = require(script.Parent.AuditLog)

local function record(command, result)
    pcall(function()
        AuditLog.record(command, result)
    end)
end

function Dispatch.register(kind, handler)
    handlers[kind] = handler
end

function Dispatch.run(command)
    if type(command) ~= "table" then
        return { ok = false, error = "command must be a table" }
    end

    local kind = command.type
    local handler = handlers[kind]
    if not handler then
        local result = { ok = false, error = "unknown command type: " .. tostring(kind) }
        record(command, result)
        return result
    end

    local ok, result = pcall(handler, command.payload or {})
    if not ok then
        local failed = { ok = false, error = tostring(result) }
        record(command, failed)
        return failed
    end
    if type(result) ~= "table" or type(result.ok) ~= "boolean" then
        local failed = { ok = false, error = "handler returned an invalid result" }
        record(command, failed)
        return failed
    end
    record(command, result)
    return result
end

return Dispatch
