local AuditLog = require(script.Parent.Parent.AuditLog)

local function historyHandler(payload)
    local action = tostring(payload.action or "list")
    if action == "list" then
        return { ok = true, data = { records = AuditLog.list() } }
    elseif action == "show" then
        if type(payload.commandId) ~= "string" or payload.commandId == "" then
            return { ok = false, error = "commandId missing" }
        end
        local record = AuditLog.show(payload.commandId)
        if not record then
            return { ok = false, error = "history record not found: " .. payload.commandId }
        end
        return { ok = true, data = record }
    elseif action == "undo" then
        if type(payload.commandId) ~= "string" or payload.commandId == "" then
            return { ok = false, error = "commandId missing" }
        end
        local result, err = AuditLog.undo(payload.commandId)
        if not result then
            return { ok = false, error = err }
        end
        return { ok = true, data = result }
    end
    return { ok = false, error = "unknown history action: " .. action }
end

return historyHandler
