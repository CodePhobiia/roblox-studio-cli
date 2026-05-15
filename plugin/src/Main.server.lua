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
        return response.data
    end
    return response
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

    local data = unwrapEnvelope(response)
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
        if not register() then
            task.wait(Config.registerRetrySeconds)
        end
    end
end

local function heartbeatLoop()
    while not stopping do
        if sessionToken then
            local _, err = Http.post(Config.bridgeUrl .. "/heartbeat/" .. sessionToken, {})
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
    local _, err = Http.post(Config.bridgeUrl .. "/result/" .. commandId, result)
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
            local command = unwrapEnvelope(response)
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
task.spawn(ensureRegistered)
task.spawn(heartbeatLoop)
task.spawn(pollLoop)

pcall(function()
    game:BindToClose(function()
        stopping = true
    end)
end)
