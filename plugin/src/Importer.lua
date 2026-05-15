local AssetService = game:GetService("AssetService")

local Importer = {}

local function sanitize(value)
    value = tostring(value or "ImportedMesh")
    value = string.gsub(value, "[<>:\"/\\|%?%*%c]", "_")
    value = string.gsub(value, "^%s+", "")
    value = string.gsub(value, "%s+$", "")
    if value == "" then
        value = "ImportedMesh"
    end
    if #value > 80 then
        value = string.sub(value, 1, 80)
    end
    return value
end

local function vectorFromArray(value)
    if type(value) ~= "table" then
        return nil
    end
    local x = tonumber(value[1])
    local y = tonumber(value[2])
    local z = tonumber(value[3])
    if not x or not y or not z then
        return nil
    end
    return Vector3.new(x, y, z)
end

local function meshBounds(vertices)
    local min = Vector3.new(math.huge, math.huge, math.huge)
    local max = Vector3.new(-math.huge, -math.huge, -math.huge)
    for _, vertex in ipairs(vertices) do
        min = Vector3.new(math.min(min.X, vertex.X), math.min(min.Y, vertex.Y), math.min(min.Z, vertex.Z))
        max = Vector3.new(math.max(max.X, vertex.X), math.max(max.Y, vertex.Y), math.max(max.Z, vertex.Z))
    end
    return min, max, (min + max) * 0.5
end

local function addTriangle(editableMesh, vertexIds, triangle, warnings, meshName)
    if type(triangle) ~= "table" then
        table.insert(warnings, meshName .. ": skipped invalid triangle")
        return false
    end

    local a = vertexIds[tonumber(triangle[1])]
    local b = vertexIds[tonumber(triangle[2])]
    local c = vertexIds[tonumber(triangle[3])]
    if not a or not b or not c then
        table.insert(warnings, meshName .. ": skipped triangle with out-of-range vertex index")
        return false
    end

    local ok, err = pcall(function()
        editableMesh:AddTriangle(a, b, c)
    end)
    if not ok then
        table.insert(warnings, meshName .. ": skipped triangle: " .. tostring(err))
        return false
    end
    return true
end

local function createMeshPart(mesh, anchored, warnings)
    local meshName = sanitize(mesh.name)
    if type(mesh.vertices) ~= "table" or type(mesh.triangles) ~= "table" then
        return nil, meshName .. ": vertices and triangles are required"
    end

    local vertices = {}
    for _, value in ipairs(mesh.vertices) do
        local vertex = vectorFromArray(value)
        if not vertex then
            return nil, meshName .. ": invalid vertex"
        end
        table.insert(vertices, vertex)
    end
    if #vertices < 3 then
        return nil, meshName .. ": at least 3 vertices are required"
    end

    local _, _, center = meshBounds(vertices)
    local okEditable, editableMeshOrErr = pcall(function()
        return AssetService:CreateEditableMesh({ FixedSize = false })
    end)
    if not okEditable or not editableMeshOrErr then
        return nil, meshName .. ": CreateEditableMesh failed: " .. tostring(editableMeshOrErr)
    end

    local editableMesh = editableMeshOrErr
    local vertexIds = {}
    for index, vertex in ipairs(vertices) do
        local okVertex, vertexIdOrErr = pcall(function()
            return editableMesh:AddVertex(vertex - center)
        end)
        if not okVertex or not vertexIdOrErr then
            editableMesh:Destroy()
            return nil, meshName .. ": AddVertex " .. tostring(index) .. " failed: " .. tostring(vertexIdOrErr)
        end
        vertexIds[index] = vertexIdOrErr
    end

    local triangleCount = 0
    for _, triangle in ipairs(mesh.triangles) do
        if addTriangle(editableMesh, vertexIds, triangle, warnings, meshName) then
            triangleCount += 1
        end
    end
    if triangleCount == 0 then
        editableMesh:Destroy()
        return nil, meshName .. ": no valid triangles"
    end

    local okPart, partOrErr = pcall(function()
        return AssetService:CreateMeshPartAsync(Content.fromObject(editableMesh), {
            CollisionFidelity = Enum.CollisionFidelity.Default,
            RenderFidelity = Enum.RenderFidelity.Automatic
        })
    end)
    if not okPart then
        okPart, partOrErr = pcall(function()
            return AssetService:CreateMeshPartAsync(Content.fromObject(editableMesh), {
                CollisionFidelity = Enum.CollisionFidelity.Default
            })
        end)
    end
    editableMesh:Destroy()
    if not okPart or not partOrErr then
        return nil, meshName .. ": CreateMeshPartAsync failed: " .. tostring(partOrErr)
    end

    local part = partOrErr
    part.Name = meshName
    part.Anchored = anchored == true
    part.CanCollide = true
    part.CanQuery = true
    part.CanTouch = true
    part.CFrame = CFrame.new(center)

    return part, nil, {
        vertexCount = #vertices,
        triangleCount = triangleCount
    }
end

local function weldParts(model, parts)
    if #parts < 2 then
        return 0
    end

    local root = parts[1]
    local count = 0
    for index = 2, #parts do
        local weld = Instance.new("WeldConstraint")
        weld.Name = "rs_Weld_" .. tostring(index - 1)
        weld.Part0 = root
        weld.Part1 = parts[index]
        weld.Parent = model
        count += 1
    end
    return count
end

function Importer.import(payload, parent)
    if type(payload) ~= "table" then
        return nil, "payload must be a table"
    end
    if type(payload.meshes) ~= "table" then
        return nil, "meshes missing"
    end

    local warnings = {}
    local root = Instance.new("Model")
    root.Name = sanitize(payload.name)

    local parts = {}
    local vertexCount = 0
    local triangleCount = 0
    for _, mesh in ipairs(payload.meshes) do
        local part, err, stats = createMeshPart(mesh, payload.anchored == true, warnings)
        if not part then
            root:Destroy()
            return nil, err
        end
        part.Parent = root
        table.insert(parts, part)
        vertexCount += stats.vertexCount
        triangleCount += stats.triangleCount
    end

    if #parts == 0 then
        root:Destroy()
        return nil, "no MeshParts were created"
    end

    local weldCount = 0
    if payload.weld ~= false then
        weldCount = weldParts(root, parts)
    end

    pcall(function()
        root.PrimaryPart = parts[1]
    end)
    root.Parent = parent

    return root, {
        rootPath = root:GetFullName(),
        meshCount = #payload.meshes,
        partCount = #parts,
        weldCount = weldCount,
        vertexCount = vertexCount,
        triangleCount = triangleCount,
        warnings = warnings
    }
end

return Importer
