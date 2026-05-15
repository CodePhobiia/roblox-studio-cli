local Dispatch = {}
local handlers = {}

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
        return { ok = false, error = "unknown command type: " .. tostring(kind) }
    end

    local ok, result = pcall(handler, command.payload or {})
    if not ok then
        return { ok = false, error = tostring(result) }
    end
    if type(result) ~= "table" or type(result.ok) ~= "boolean" then
        return { ok = false, error = "handler returned an invalid result" }
    end
    return result
end

return Dispatch
