local Inspector = {}

local ASSET_PROPERTIES = {
    AnimationId = "animation",
    BottomImage = "image",
    Image = "image",
    MidImage = "image",
    MeshId = "mesh",
    SoundId = "audio",
    TopImage = "image",
    Texture = "image",
    TextureID = "image",
    TextureId = "image"
}

local REF_PROPERTIES = {
    Weld = { "Part0", "Part1" },
    ManualWeld = { "Part0", "Part1" },
    Motor = { "Part0", "Part1" },
    Motor6D = { "Part0", "Part1" },
    Snap = { "Part0", "Part1" },
    WeldConstraint = { "Part0", "Part1" },
    NoCollisionConstraint = { "Part0", "Part1" },
    ObjectValue = { "Value" },
    SurfaceGui = { "Adornee" },
    BillboardGui = { "Adornee" },
    Beam = { "Attachment0", "Attachment1" },
    Trail = { "Attachment0", "Attachment1" },
    HingeConstraint = { "Attachment0", "Attachment1" },
    BallSocketConstraint = { "Attachment0", "Attachment1" },
    RigidConstraint = { "Attachment0", "Attachment1" },
    RodConstraint = { "Attachment0", "Attachment1" },
    PrismaticConstraint = { "Attachment0", "Attachment1" },
    CylindricalConstraint = { "Attachment0", "Attachment1" },
    UniversalConstraint = { "Attachment0", "Attachment1" },
    AlignPosition = { "Attachment0", "Attachment1" },
    AlignOrientation = { "Attachment0", "Attachment1" },
    LinearVelocity = { "Attachment0", "Attachment1" },
    AngularVelocity = { "Attachment0" },
    VectorForce = { "Attachment0" },
    Torque = { "Attachment0" },
    LineForce = { "Attachment0", "Attachment1" },
    RopeConstraint = { "Attachment0", "Attachment1" },
    SpringConstraint = { "Attachment0", "Attachment1" }
}

local RIGID_JOINT_CLASSES = {
    Weld = true,
    ManualWeld = true,
    Motor = true,
    Motor6D = true,
    Snap = true,
    WeldConstraint = true
}

local function fullName(instance)
    local ok, value = pcall(function()
        return instance:GetFullName()
    end)
    return ok and value or instance.Name
end

local function descendantsInclusive(root)
    local items = { root }
    for _, descendant in ipairs(root:GetDescendants()) do
        table.insert(items, descendant)
    end
    return items
end

local function addDiagnostic(out, severity, rule, instance, property, message, fixId)
    table.insert(out.diagnostics, {
        severity = severity,
        rule = rule,
        path = fullName(instance),
        property = property,
        message = message,
        fixId = fixId
    })
    out.summary[severity] = (out.summary[severity] or 0) + 1
end

local function ruleSet(rules)
    local set = {}
    if type(rules) ~= "table" or #rules == 0 then
        set.all = true
        return set
    end
    for _, rule in ipairs(rules) do
        set[string.lower(tostring(rule))] = true
    end
    return set
end

local function hasRule(set, name)
    return set.all or set[name] == true
end

local function readProperty(instance, property)
    local ok, value = pcall(function()
        return instance[property]
    end)
    if ok then
        return value, nil
    end
    return nil, tostring(value)
end

local function isInside(value, root)
    return typeof(value) == "Instance" and (value == root or value:IsDescendantOf(root))
end

local function validateReferences(root, out)
    for _, instance in ipairs(descendantsInclusive(root)) do
        local props = REF_PROPERTIES[instance.ClassName]
        if props then
            for _, property in ipairs(props) do
                local value, err = readProperty(instance, property)
                if err then
                    table.insert(out.warnings, fullName(instance) .. "." .. property .. ": " .. err)
                elseif value == nil then
                    local severity = "warn"
                    if property == "Part0" or property == "Part1" or property == "Attachment0" or property == "Attachment1" then
                        severity = "fail"
                    end
                    addDiagnostic(out, severity, "ref.missing", instance, property, property .. " is nil", "repair-tool")
                elseif typeof(value) == "Instance" and not isInside(value, root) then
                    addDiagnostic(out, "warn", "ref.external", instance, property, property .. " points outside the validated subtree", nil)
                end
            end
        end
    end
end

local function collectBaseParts(tool)
    local parts = {}
    for _, descendant in ipairs(tool:GetDescendants()) do
        if descendant:IsA("BasePart") then
            table.insert(parts, descendant)
        end
    end
    return parts
end

local function rigidJointParts(joint)
    if not RIGID_JOINT_CLASSES[joint.ClassName] then
        return nil, nil
    end
    local p0 = readProperty(joint, "Part0")
    local p1 = readProperty(joint, "Part1")
    if typeof(p0) == "Instance" and p0:IsA("BasePart") and typeof(p1) == "Instance" and p1:IsA("BasePart") then
        return p0, p1
    end
    return nil, nil
end

local function connectedToHandle(tool, handle, parts)
    local graph = {}
    for _, part in ipairs(parts) do
        graph[part] = {}
    end
    for _, descendant in ipairs(tool:GetDescendants()) do
        local p0, p1 = rigidJointParts(descendant)
        if p0 and p1 and graph[p0] and graph[p1] then
            table.insert(graph[p0], p1)
            table.insert(graph[p1], p0)
        end
    end

    local seen = {}
    local queue = { handle }
    seen[handle] = true
    local index = 1
    while index <= #queue do
        local current = queue[index]
        index += 1
        for _, nextPart in ipairs(graph[current] or {}) do
            if not seen[nextPart] then
                seen[nextPart] = true
                table.insert(queue, nextPart)
            end
        end
    end
    return seen
end

local function validateTool(tool, out)
    local handle = tool:FindFirstChild("Handle")
    if not handle or not handle:IsA("BasePart") then
        local requiresHandle = true
        pcall(function()
            requiresHandle = tool.RequiresHandle
        end)
        if requiresHandle then
            addDiagnostic(out, "fail", "tool.handle.missing", tool, "Handle", "Tool requires a BasePart child named Handle", "repair-tool")
        else
            addDiagnostic(out, "info", "tool.handle.optional", tool, "Handle", "Tool has RequiresHandle=false and no Handle", nil)
        end
        return
    end

    local duplicateHandles = 0
    for _, child in ipairs(tool:GetChildren()) do
        if child.Name == "Handle" then
            duplicateHandles += 1
        end
    end
    if duplicateHandles > 1 then
        addDiagnostic(out, "warn", "tool.handle.ambiguous", tool, "Handle", "Tool has multiple direct children named Handle", "repair-tool")
    end

    local parts = collectBaseParts(tool)
    local connected = connectedToHandle(tool, handle, parts)
    for _, part in ipairs(parts) do
        local anchored = false
        pcall(function()
            anchored = part.Anchored
        end)
        if anchored then
            addDiagnostic(out, "fail", "tool.part.anchored", part, "Anchored", "Tool BasePart is anchored", "repair-tool")
        end
        if part ~= handle and not connected[part] then
            addDiagnostic(out, "fail", "tool.part.disconnected", part, nil, "BasePart is not rigidly connected to Handle", "repair-tool")
        end
    end
    addDiagnostic(out, "info", "tool.parts", tool, nil, "Tool contains " .. tostring(#parts) .. " BasePart descendant(s)", nil)
end

local function validateTools(root, out)
    for _, instance in ipairs(descendantsInclusive(root)) do
        if instance:IsA("Tool") then
            validateTool(instance, out)
        end
    end
end

local function validateAssets(root, out)
    for _, instance in ipairs(descendantsInclusive(root)) do
        if instance:IsA("MeshPart") then
            local meshId = readProperty(instance, "MeshId")
            if type(meshId) ~= "string" or meshId == "" then
                addDiagnostic(out, "warn", "asset.mesh.missing", instance, "MeshId", "MeshPart has an empty MeshId", nil)
            end
        elseif instance:IsA("ImageLabel") or instance:IsA("ImageButton") then
            local image = readProperty(instance, "Image")
            local imageContent = readProperty(instance, "ImageContent")
            if (type(image) ~= "string" or image == "") and imageContent == nil then
                addDiagnostic(out, "warn", "asset.image.missing", instance, "Image", "Image object has no Image or ImageContent", nil)
            end
        elseif instance:IsA("Sound") then
            local soundId = readProperty(instance, "SoundId")
            if type(soundId) ~= "string" or soundId == "" then
                addDiagnostic(out, "warn", "asset.sound.missing", instance, "SoundId", "Sound has an empty SoundId", nil)
            end
        end
    end
end

local function validateDuplicateNames(root, out)
    for _, instance in ipairs(descendantsInclusive(root)) do
        local counts = {}
        for _, child in ipairs(instance:GetChildren()) do
            counts[child.Name] = (counts[child.Name] or 0) + 1
        end
        for name, count in pairs(counts) do
            if count > 1 then
                addDiagnostic(out, "warn", "path.duplicate-name", instance, nil, tostring(count) .. " children are named '" .. tostring(name) .. "'", nil)
            end
        end
    end
end

function Inspector.validate(root, rules)
    local out = {
        path = fullName(root),
        summary = { fail = 0, warn = 0, info = 0 },
        diagnostics = {},
        warnings = {}
    }
    local enabled = ruleSet(rules)
    if hasRule(enabled, "refs") or hasRule(enabled, "welds") then
        validateReferences(root, out)
    end
    if hasRule(enabled, "tool") then
        validateTools(root, out)
    end
    if hasRule(enabled, "assets") then
        validateAssets(root, out)
    end
    if hasRule(enabled, "paths") or hasRule(enabled, "transfer") then
        validateDuplicateNames(root, out)
    end
    return out
end

local function countSubtree(root)
    return 1 + #root:GetDescendants()
end

local function maybeAsset(instance, property)
    local value = readProperty(instance, property)
    if type(value) == "string" and value ~= "" then
        return value
    end
    return nil
end

function Inspector.snapshot(root, includePaths)
    local classCounts = {}
    local scriptCounts = {}
    local assetRefs = {}
    local duplicateNames = {}
    local topSubtrees = {}
    local paths = {}
    local total = 0
    local maxDepth = 0
    local toolCount = 0
    local meshPartCount = 0
    local uiCount = 0
    local remoteCount = 0
    local warnings = {}

    local function walk(instance, depth)
        total += 1
        maxDepth = math.max(maxDepth, depth)
        classCounts[instance.ClassName] = (classCounts[instance.ClassName] or 0) + 1
        if includePaths then
            table.insert(paths, fullName(instance))
        end
        if instance:IsA("Script") or instance:IsA("LocalScript") or instance:IsA("ModuleScript") then
            scriptCounts[instance.ClassName] = (scriptCounts[instance.ClassName] or 0) + 1
        end
        if instance:IsA("Tool") then
            toolCount += 1
        end
        if instance:IsA("MeshPart") then
            meshPartCount += 1
        end
        if instance:IsA("GuiObject") or instance:IsA("LayerCollector") then
            uiCount += 1
        end
        if instance:IsA("RemoteEvent") or instance:IsA("RemoteFunction") then
            remoteCount += 1
        end

        for property, kind in pairs(ASSET_PROPERTIES) do
            local assetUri = maybeAsset(instance, property)
            if assetUri then
                table.insert(assetRefs, {
                    path = fullName(instance),
                    property = property,
                    assetUri = assetUri,
                    assetKind = kind
                })
            end
        end

        local siblingCounts = {}
        for _, child in ipairs(instance:GetChildren()) do
            siblingCounts[child.Name] = (siblingCounts[child.Name] or 0) + 1
        end
        for name, count in pairs(siblingCounts) do
            if count > 1 then
                table.insert(duplicateNames, {
                    parentPath = fullName(instance),
                    name = name,
                    count = count
                })
            end
        end

        table.insert(topSubtrees, {
            path = fullName(instance),
            count = countSubtree(instance)
        })

        for _, child in ipairs(instance:GetChildren()) do
            walk(child, depth + 1)
        end
    end

    walk(root, 0)
    table.sort(topSubtrees, function(a, b)
        return a.count > b.count
    end)
    while #topSubtrees > 10 do
        table.remove(topSubtrees)
    end

    return {
        rootPath = fullName(root),
        totalInstances = total,
        maxDepth = maxDepth,
        classCounts = classCounts,
        scriptCounts = scriptCounts,
        toolCount = toolCount,
        meshPartCount = meshPartCount,
        uiCount = uiCount,
        remoteCount = remoteCount,
        assetReferences = assetRefs,
        duplicateSiblingNames = duplicateNames,
        topSubtrees = topSubtrees,
        paths = paths,
        warnings = warnings
    }
end

local function allToolJoints(tool)
    local joints = {}
    for _, descendant in ipairs(tool:GetDescendants()) do
        if RIGID_JOINT_CLASSES[descendant.ClassName] then
            table.insert(joints, descendant)
        end
    end
    return joints
end

function Inspector.repairTool(tool, options)
    options = options or {}
    local dryRun = options.dryRun == true
    local handleName = type(options.handle) == "string" and options.handle or "Handle"
    local handle = tool:FindFirstChild(handleName)
    if not handle or not handle:IsA("BasePart") then
        return nil, "Tool has no BasePart child named " .. handleName
    end

    local parts = collectBaseParts(tool)
    local result = {
        path = fullName(tool),
        dryRun = dryRun,
        partsFound = #parts,
        validJointsPreserved = 0,
        brokenJointsFound = 0,
        weldsCreated = 0,
        physicsPropertiesChanged = 0,
        warnings = {}
    }

    for _, joint in ipairs(allToolJoints(tool)) do
        local p0, p1 = rigidJointParts(joint)
        if p0 and p1 then
            result.validJointsPreserved += 1
        else
            result.brokenJointsFound += 1
            if options.replaceBroken == true and not dryRun then
                joint:Destroy()
            end
        end
    end

    local connected = connectedToHandle(tool, handle, parts)
    for _, part in ipairs(parts) do
        if part ~= handle and not connected[part] then
            result.weldsCreated += 1
            if not dryRun then
                local weld = Instance.new("WeldConstraint")
                weld.Name = "rs_HandleWeld_" .. part.Name
                weld.Part0 = handle
                weld.Part1 = part
                weld.Parent = tool
            end
        end

        if options.physicsFix ~= false then
            local changed = false
            if part.Anchored ~= false then
                changed = true
                if not dryRun then
                    part.Anchored = false
                end
            end
            if part ~= handle then
                local desiredCollision = options.collision
                if desiredCollision == nil then
                    desiredCollision = false
                end
                if part.CanCollide ~= desiredCollision then
                    changed = true
                    if not dryRun then
                        part.CanCollide = desiredCollision
                    end
                end
                if options.massless ~= nil and part.Massless ~= options.massless then
                    changed = true
                    if not dryRun then
                        part.Massless = options.massless
                    end
                end
            end
            if changed then
                result.physicsPropertiesChanged += 1
            end
        end
    end

    return result, nil
end

return Inspector
