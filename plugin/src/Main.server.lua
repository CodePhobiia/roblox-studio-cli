local HttpService = game:GetService("HttpService")
local Config = require(script.Parent.Config)
local Http = require(script.Parent.Http)
local Dispatch = require(script.Parent.Dispatch)
local AutopilotReview = require(script.Parent.AutopilotReview)

local PROTOCOL_VERSION = 5
local PLUGIN_VERSION = "0.4.0"
local CAPABILITIES = {
    "exec",
    "read",
    "export",
    "importAsset",
    "importImage",
    "importUploaded",
    "importUiPack",
    "importAudio",
    "validate",
    "repairTool",
    "snapshot",
    "create",
    "upsertFiles",
    "applyPlan",
    "packageUpdate",
    "history",
    "deps",
    "serialize",
    "deserialize",
    "autopilotReview"
}

Dispatch.register("exec", require(script.Parent.Handlers.Exec))
Dispatch.register("read", require(script.Parent.Handlers.Read))
Dispatch.register("export", require(script.Parent.Handlers.Export))
Dispatch.register("importAsset", require(script.Parent.Handlers.ImportAsset))
Dispatch.register("importImage", require(script.Parent.Handlers.ImportImage))
Dispatch.register("importUploaded", require(script.Parent.Handlers.ImportUploaded))
Dispatch.register("importUiPack", require(script.Parent.Handlers.ImportUiPack))
Dispatch.register("importAudio", require(script.Parent.Handlers.ImportAudio))
Dispatch.register("validate", require(script.Parent.Handlers.Validate))
Dispatch.register("repairTool", require(script.Parent.Handlers.RepairTool))
Dispatch.register("snapshot", require(script.Parent.Handlers.Snapshot))
Dispatch.register("create", require(script.Parent.Handlers.Create))
Dispatch.register("upsertFiles", require(script.Parent.Handlers.UpsertFiles))
Dispatch.register("applyPlan", require(script.Parent.Handlers.ApplyPlan))
Dispatch.register("packageUpdate", require(script.Parent.Handlers.PackageUpdate))
Dispatch.register("history", require(script.Parent.Handlers.History))
Dispatch.register("deps", require(script.Parent.Handlers.Deps))
Dispatch.register("serialize", require(script.Parent.Handlers.Serialize))
Dispatch.register("deserialize", require(script.Parent.Handlers.Deserialize))
Dispatch.register("autopilotReview", AutopilotReview.handle)

pcall(function()
    AutopilotReview.setup(plugin)
end)

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
        placeFilePath = getPlaceFilePath(),
        protocolVersion = PROTOCOL_VERSION,
        pluginVersion = PLUGIN_VERSION,
        capabilities = CAPABILITIES
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
