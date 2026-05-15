local HttpService = game:GetService("HttpService")
local Config = require(script.Parent.Config)
local Http = require(script.Parent.Http)
local Dispatch = require(script.Parent.Dispatch)

Dispatch.register("exec", require(script.Parent.Handlers.Exec))
Dispatch.register("read", require(script.Parent.Handlers.Read))
Dispatch.register("serialize", require(script.Parent.Handlers.Serialize))
Dispatch.register("deserialize", require(script.Parent.Handlers.Deserialize))

local studioId = HttpService:GenerateGUID(false)
local sessionToken = nil
local registering = false
local stopping = false

local function getPlaceFilePath()
    local ok, value = pcall(function()
        return game:GetService("DataModelService"):GetFilePath()
    end)
    if ok then
        return value
    end
    return nil
end

local function unwrapEnvelope(response)
    if type(response) == "table" and response.ok == true then
        return response.data, nil
    end
    if type(response) == "table" and response.ok == false then
        return nil, tostring(response.error or "bridge returned ok=false")
    end
    return response, nil
end

local function register()
    local response, err = Http.post(Config.bridgeUrl .. "/register", {
        id = studioId,
        name = game.Name,
        placeFilePath = getPlaceFilePath()
    })
    if not response then
        warn("[rs-bridge-plugin] register failed: " .. tostring(err))
        return false
    end

    local data, envelopeErr = unwrapEnvelope(response)
    if envelopeErr then
        warn("[rs-bridge-plugin] register rejected: " .. envelopeErr)
        return false
    end
    if type(data) == "table" and type(data.sessionToken) == "string" then
        sessionToken = data.sessionToken
        print("[rs-bridge-plugin] Registered as '" .. game.Name .. "'")
        return true
    end

    warn("[rs-bridge-plugin] register returned an invalid response")
    return false
end

local function ensureRegistered()
    while not stopping and not sessionToken do
        if registering then
            task.wait(0.1)
        else
            registering = true
            local ok = register()
            registering = false
            if not ok then
                task.wait(Config.registerRetrySeconds)
            end
        end
    end
end

local function heartbeatLoop()
    while not stopping do
        if sessionToken then
            local response, err = Http.post(Config.bridgeUrl .. "/heartbeat/" .. sessionToken, {})
            if not err then
                local _, envelopeErr = unwrapEnvelope(response)
                err = envelopeErr
            end
            if err then
                warn("[rs-bridge-plugin] heartbeat failed: " .. tostring(err))
                sessionToken = nil
                ensureRegistered()
            end
        end
        task.wait(Config.heartbeatSeconds)
    end
end

local function postResult(commandId, result)
    local response, err = Http.post(Config.bridgeUrl .. "/result/" .. commandId, result)
    if not err then
        local _, envelopeErr = unwrapEnvelope(response)
        err = envelopeErr
    end
    if err then
        warn("[rs-bridge-plugin] result post failed: " .. tostring(err))
    end
end

local function pollLoop()
    while not stopping do
        if not sessionToken then
            ensureRegistered()
        end

        local response, err = Http.get(Config.bridgeUrl .. "/poll/" .. sessionToken)
        if response then
            local command, envelopeErr = unwrapEnvelope(response)
            if envelopeErr then
                warn("[rs-bridge-plugin] poll rejected: " .. tostring(envelopeErr))
                sessionToken = nil
                task.wait(Config.registerRetrySeconds)
                continue
            end
            if type(command) == "table" and command.commandId then
                local result = Dispatch.run(command)
                postResult(command.commandId, result)
            end
        elseif err then
            warn("[rs-bridge-plugin] poll failed: " .. tostring(err))
            sessionToken = nil
            task.wait(Config.registerRetrySeconds)
        end

        task.wait(Config.pollDelaySeconds)
    end
end

print("[rs-bridge-plugin] Loaded")
task.spawn(heartbeatLoop)
task.spawn(pollLoop)

pcall(function()
    game:BindToClose(function()
        stopping = true
    end)
end)
