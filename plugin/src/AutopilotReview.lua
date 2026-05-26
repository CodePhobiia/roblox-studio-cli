local AutopilotReview = {}

local latestRun = nil
local widget = nil
local contentLabel = nil

local function valueOrDash(value)
    if value == nil or value == "" then
        return "-"
    end
    return tostring(value)
end

local function appendCounts(lines, counts)
    if type(counts) ~= "table" then
        return
    end
    table.insert(lines, "")
    table.insert(lines, "Operations")
    local keys = {}
    for key in pairs(counts) do
        table.insert(keys, tostring(key))
    end
    table.sort(keys)
    for _, key in ipairs(keys) do
        table.insert(lines, "  " .. key .. ": " .. tostring(counts[key]))
    end
end

local function appendList(lines, title, values, limit)
    if type(values) ~= "table" or #values == 0 then
        return
    end
    table.insert(lines, "")
    table.insert(lines, title)
    for index, value in ipairs(values) do
        if index > limit then
            table.insert(lines, "  ... " .. tostring(#values - limit) .. " more")
            break
        end
        table.insert(lines, "  " .. tostring(value))
    end
end

local function appendActions(lines, values, limit)
    if type(values) ~= "table" or #values == 0 then
        return
    end
    table.insert(lines, "")
    table.insert(lines, "Next actions")
    for index, action in ipairs(values) do
        if index > limit then
            table.insert(lines, "  ... " .. tostring(#values - limit) .. " more")
            break
        end
        if type(action) == "table" then
            table.insert(lines, "  " .. valueOrDash(action.priority) .. ". " .. valueOrDash(action.title))
            if action.command then
                table.insert(lines, "     " .. tostring(action.command))
            end
        else
            table.insert(lines, "  " .. tostring(action))
        end
    end
end

local function appendAction(lines, title, action)
    if type(action) ~= "table" then
        return
    end
    table.insert(lines, "")
    table.insert(lines, title)
    table.insert(lines, "  " .. valueOrDash(action.title))
    if action.command then
        table.insert(lines, "     " .. tostring(action.command))
    end
    if action.reason then
        table.insert(lines, "     " .. tostring(action.reason))
    end
end

local function appendRecords(lines, title, values, limit)
    if type(values) ~= "table" or #values == 0 then
        return
    end
    table.insert(lines, "")
    table.insert(lines, title)
    for index, value in ipairs(values) do
        if index > limit then
            table.insert(lines, "  ... " .. tostring(#values - limit) .. " more")
            break
        end
        if type(value) == "table" then
            local label = value.title or value.id or value.kind or value.path
            table.insert(lines, "  " .. valueOrDash(label))
            if value.status then
                table.insert(lines, "     Status: " .. valueOrDash(value.status))
            end
            if value.path then
                table.insert(lines, "     " .. tostring(value.path))
            end
            if value.reason then
                table.insert(lines, "     " .. tostring(value.reason))
            end
        else
            table.insert(lines, "  " .. tostring(value))
        end
    end
end

local function render()
    if not contentLabel then
        return
    end

    local lines = {}
    if type(latestRun) ~= "table" then
        table.insert(lines, "No Autopilot run has been published to this Studio session yet.")
        table.insert(lines, "")
        table.insert(lines, "Run `rs autopilot preview --live` or `rs autopilot apply` to send a review summary here.")
    else
        table.insert(lines, "Run: " .. valueOrDash(latestRun.runId))
        table.insert(lines, "Status: " .. valueOrDash(latestRun.status))
        table.insert(lines, "Scope: " .. valueOrDash(latestRun.scope))
        table.insert(lines, "Risk: " .. valueOrDash(type(latestRun.risk) == "table" and latestRun.risk.level or nil))
        table.insert(lines, "Updated: " .. valueOrDash(latestRun.updatedAt))
        table.insert(lines, "")
        table.insert(lines, "Prompt")
        table.insert(lines, valueOrDash(latestRun.prompt))
        if latestRun.planHash or latestRun.previewIntegrityStatus or latestRun.alphaPacketStatus or latestRun.applyCommand then
            table.insert(lines, "")
            table.insert(lines, "Gate proof")
            table.insert(lines, "  Plan hash: " .. valueOrDash(latestRun.planHash))
            table.insert(lines, "  Preview integrity: " .. valueOrDash(latestRun.previewIntegrityStatus))
            table.insert(lines, "  Alpha packet: " .. valueOrDash(latestRun.alphaPacketStatus))
            if latestRun.applyCommand then
                table.insert(lines, "  Apply: " .. tostring(latestRun.applyCommand))
            end
        end
        appendCounts(lines, latestRun.operationCounts)
        if latestRun.companionStatus or latestRun.reviewPackStatus or latestRun.arcStatus or latestRun.bestFriendStatus or latestRun.bestFriendPilotStatus or latestRun.bestFriendRunnerStatus then
            table.insert(lines, "")
            table.insert(lines, "AI handoff")
            table.insert(lines, "  Companion: " .. valueOrDash(latestRun.companionStatus))
            table.insert(lines, "  Review pack: " .. valueOrDash(latestRun.reviewPackStatus))
            table.insert(lines, "  Best friend: " .. valueOrDash(latestRun.bestFriendStatus))
            table.insert(lines, "  Pilot: " .. valueOrDash(latestRun.bestFriendPilotStatus))
            table.insert(lines, "  Runner: " .. valueOrDash(latestRun.bestFriendRunnerStatus))
            table.insert(lines, "  Setup ready: " .. valueOrDash(latestRun.setupReady))
            table.insert(lines, "  Recommended: " .. valueOrDash(latestRun.recommendedCandidateId))
        end
        if latestRun.arcStatus then
            table.insert(lines, "")
            table.insert(lines, "AI companion arc")
            table.insert(lines, "  Status: " .. valueOrDash(latestRun.arcStatus))
            table.insert(lines, "  Phase: " .. valueOrDash(latestRun.arcPhase))
            table.insert(lines, "  Verdict: " .. valueOrDash(latestRun.arcVerdict))
            table.insert(lines, "  Wow session: " .. valueOrDash(latestRun.wowSessionStatus))
            table.insert(lines, "  Demo session: " .. valueOrDash(latestRun.demoSessionStatus))
            table.insert(lines, "  Demo ready: " .. valueOrDash(latestRun.arcReadyForCreatorDemo))
            table.insert(lines, "  Reply ready: " .. valueOrDash(latestRun.arcReadyForPostDemoReply))
            table.insert(lines, "  Demo title: " .. valueOrDash(latestRun.demoTitle))
            if type(latestRun.selectedWowMoment) == "table" then
                table.insert(lines, "  Moment: " .. valueOrDash(latestRun.selectedWowMoment.title))
                table.insert(lines, "  Player beat: " .. valueOrDash(latestRun.selectedWowMoment.playerMoment))
            end
            if latestRun.creatorMessage then
                table.insert(lines, "")
                table.insert(lines, "Creator demo message")
                table.insert(lines, tostring(latestRun.creatorMessage))
            end
            if latestRun.replyMessage then
                table.insert(lines, "")
                table.insert(lines, "Checked post-demo reply")
                table.insert(lines, tostring(latestRun.replyMessage))
            end
        end
        if latestRun.bestFriendStatus then
            table.insert(lines, "")
            table.insert(lines, "AI best friend context")
            table.insert(lines, "  Status: " .. valueOrDash(latestRun.bestFriendStatus))
            table.insert(lines, "  Relationship: " .. valueOrDash(latestRun.bestFriendRelationshipMode))
            table.insert(lines, "  Task pack: " .. valueOrDash(latestRun.bestFriendTaskPackStatus))
            table.insert(lines, "  Preflight: " .. valueOrDash(latestRun.bestFriendLaunchPreflightStatus))
            table.insert(lines, "  Opening claim: " .. valueOrDash(latestRun.bestFriendOpeningReplyClaim))
            appendAction(lines, "First safe action", latestRun.bestFriendFirstSafeAction)
            appendAction(lines, "Protected first action", latestRun.bestFriendFirstActAction)
            if latestRun.bestFriendOpeningReply then
                table.insert(lines, "")
                table.insert(lines, "Checked opening reply")
                table.insert(lines, tostring(latestRun.bestFriendOpeningReply))
            end
            appendRecords(lines, "Read first", latestRun.bestFriendReadOrder, 5)
            appendRecords(lines, "Context cards", latestRun.bestFriendContextCards, 5)
        end
        if latestRun.bestFriendPilotStatus then
            table.insert(lines, "")
            table.insert(lines, "AI co-pilot")
            table.insert(lines, "  Status: " .. valueOrDash(latestRun.bestFriendPilotStatus))
            table.insert(lines, "  Mode: " .. valueOrDash(latestRun.bestFriendPilotMode))
            table.insert(lines, "  Verdict: " .. valueOrDash(latestRun.bestFriendPilotVerdict))
            table.insert(lines, "  Mentor: " .. valueOrDash(latestRun.bestFriendPilotMentorStatus))
            table.insert(lines, "  First-turn: " .. valueOrDash(latestRun.bestFriendPilotFirstTurnStatus))
            table.insert(lines, "  Rescue: " .. valueOrDash(latestRun.bestFriendPilotRescueStatus))
            table.insert(lines, "  Reply: " .. valueOrDash(latestRun.bestFriendPilotReplyStatus))
            table.insert(lines, "  Protected ready: " .. valueOrDash(latestRun.bestFriendPilotProtectedActionReady))
            if latestRun.bestFriendPilotSelectedCommand then
                table.insert(lines, "")
                table.insert(lines, "Selected co-pilot command")
                table.insert(lines, tostring(latestRun.bestFriendPilotSelectedCommand))
            end
            if latestRun.bestFriendPilotCompanionMessage then
                table.insert(lines, "")
                table.insert(lines, "Checked companion message")
                table.insert(lines, tostring(latestRun.bestFriendPilotCompanionMessage))
            end
        end
        if latestRun.bestFriendRunnerStatus then
            table.insert(lines, "")
            table.insert(lines, "AI runner")
            table.insert(lines, "  Status: " .. valueOrDash(latestRun.bestFriendRunnerStatus))
            table.insert(lines, "  Verdict: " .. valueOrDash(latestRun.bestFriendRunnerVerdict))
            table.insert(lines, "  Stopped: " .. valueOrDash(latestRun.bestFriendRunnerStoppedReason))
            table.insert(lines, "  Steps: " .. valueOrDash(latestRun.bestFriendRunnerExecutedSteps) .. " / " .. valueOrDash(latestRun.bestFriendRunnerMaxSteps))
            table.insert(lines, "  Last branch: " .. valueOrDash(latestRun.bestFriendRunnerLastBranch))
            if latestRun.bestFriendRunnerCompanionMessage then
                table.insert(lines, "")
                table.insert(lines, "Runner companion message")
                table.insert(lines, tostring(latestRun.bestFriendRunnerCompanionMessage))
            end
            appendRecords(lines, "Runner steps", latestRun.bestFriendRunnerSteps, 4)
        end
        if latestRun.agentBrief then
            table.insert(lines, "")
            table.insert(lines, "Agent brief")
            table.insert(lines, tostring(latestRun.agentBrief))
        end
        appendList(lines, "Blockers", latestRun.blockers, 8)
        appendList(lines, "Changed paths", latestRun.changedPaths, 12)
        appendList(lines, "Warnings", latestRun.warnings, 8)
        appendActions(lines, latestRun.nextActions, 6)
        appendList(lines, "Safe to say", latestRun.safeToSay, 5)
        appendList(lines, "Do not say", latestRun.doNotSay, 6)
        if latestRun.rollbackArtifact then
            table.insert(lines, "")
            table.insert(lines, "Rollback")
            table.insert(lines, "  Artifact: " .. tostring(latestRun.rollbackArtifact))
            if type(latestRun.rollbackRestore) == "table" then
                table.insert(lines, "  Restore: " .. valueOrDash(latestRun.rollbackRestore.status))
            end
        end
        if type(latestRun.smoke) == "table" then
            table.insert(lines, "")
            table.insert(lines, "Smoke")
            table.insert(lines, "  Suite: " .. valueOrDash(latestRun.smoke.suite))
            table.insert(lines, "  Status: " .. (latestRun.smoke.ok and "PASS" or "FAIL"))
            if latestRun.smoke.artifact then
                table.insert(lines, "  Artifact: " .. tostring(latestRun.smoke.artifact))
            end
        end
        if latestRun.error then
            table.insert(lines, "")
            table.insert(lines, "Error")
            table.insert(lines, tostring(latestRun.error))
        end
        if type(latestRun.artifacts) == "table" then
            table.insert(lines, "")
            table.insert(lines, "Report")
            table.insert(lines, valueOrDash(latestRun.artifacts.report))
        end
    end

    contentLabel.Text = table.concat(lines, "\n")
    task.defer(function()
        if contentLabel and contentLabel.Parent then
            contentLabel.Parent.CanvasSize = UDim2.fromOffset(0, contentLabel.TextBounds.Y + 32)
        end
    end)
end

function AutopilotReview.setup(pluginObject)
    if not pluginObject or widget then
        return
    end

    local toolbar = pluginObject:CreateToolbar("rs")
    local button = toolbar:CreateButton("AutopilotReview", "Show rs Autopilot review", "")
    button.ClickableWhenViewportHidden = true

    local info = DockWidgetPluginGuiInfo.new(
        Enum.InitialDockState.Right,
        false,
        false,
        380,
        520,
        280,
        240
    )
    widget = pluginObject:CreateDockWidgetPluginGui("rsAutopilotReview", info)
    widget.Title = "rs Autopilot"

    local root = Instance.new("Frame")
    root.BackgroundColor3 = Color3.fromRGB(24, 26, 30)
    root.BorderSizePixel = 0
    root.Size = UDim2.fromScale(1, 1)
    root.Parent = widget

    local title = Instance.new("TextLabel")
    title.BackgroundTransparency = 1
    title.Font = Enum.Font.SourceSansBold
    title.Text = "Autopilot Review"
    title.TextColor3 = Color3.fromRGB(242, 244, 248)
    title.TextSize = 18
    title.TextXAlignment = Enum.TextXAlignment.Left
    title.Position = UDim2.fromOffset(16, 12)
    title.Size = UDim2.new(1, -32, 0, 28)
    title.Parent = root

    local scroll = Instance.new("ScrollingFrame")
    scroll.BackgroundColor3 = Color3.fromRGB(32, 35, 40)
    scroll.BorderSizePixel = 0
    scroll.CanvasSize = UDim2.fromOffset(0, 0)
    scroll.Position = UDim2.fromOffset(12, 52)
    scroll.ScrollBarThickness = 8
    scroll.Size = UDim2.new(1, -24, 1, -64)
    scroll.Parent = root

    contentLabel = Instance.new("TextLabel")
    contentLabel.BackgroundTransparency = 1
    contentLabel.Font = Enum.Font.Code
    contentLabel.TextColor3 = Color3.fromRGB(224, 229, 236)
    contentLabel.TextSize = 14
    contentLabel.TextWrapped = true
    contentLabel.TextXAlignment = Enum.TextXAlignment.Left
    contentLabel.TextYAlignment = Enum.TextYAlignment.Top
    contentLabel.Position = UDim2.fromOffset(12, 12)
    contentLabel.Size = UDim2.new(1, -24, 0, 0)
    contentLabel.AutomaticSize = Enum.AutomaticSize.Y
    contentLabel.Parent = scroll

    button.Click:Connect(function()
        widget.Enabled = not widget.Enabled
        render()
    end)

    render()
end

function AutopilotReview.handle(payload)
    local action = tostring(payload.action or "get")
    if action == "set" then
        if type(payload.run) ~= "table" then
            return { ok = false, error = "run missing" }
        end
        latestRun = payload.run
        render()
        return { ok = true, data = { status = "stored", runId = tostring(latestRun.runId or "") } }
    elseif action == "clear" then
        latestRun = nil
        render()
        return { ok = true, data = { status = "cleared" } }
    elseif action == "get" then
        return { ok = true, data = { status = latestRun and "available" or "empty", run = latestRun } }
    end
    return { ok = false, error = "unknown autopilotReview action: " .. action }
end

return AutopilotReview
