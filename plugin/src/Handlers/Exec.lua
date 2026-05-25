local Encoders = require(script.Parent.Parent.PropertyEncoders)

local function execHandler(payload)
    if payload.allowDangerousExec ~= true then
        return { ok = false, error = "exec requires allowDangerousExec=true" }
    end

    if type(payload.lua) ~= "string" then
        return { ok = false, error = "lua payload missing" }
    end

    local chunk, compileErr = loadstring(payload.lua)
    if not chunk then
        return { ok = false, error = tostring(compileErr) }
    end

    local results = table.pack(pcall(chunk))
    local ok = table.remove(results, 1)
    results.n = results.n - 1
    if not ok then
        return { ok = false, error = tostring(results[1]) }
    end

    local data
    if results.n <= 1 then
        data = Encoders.encode(results[1])
    else
        data = {}
        for i = 1, results.n do
            data[i] = Encoders.encode(results[i])
        end
    end
    return { ok = true, data = data }
end

return execHandler
