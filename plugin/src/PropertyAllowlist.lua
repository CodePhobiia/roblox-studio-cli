local Allowlist = {}

local common = { "Name", "Archivable" }

local basePart = {
    "Anchored", "CanCollide", "CanTouch", "CanQuery", "CastShadow", "Color", "BrickColor",
    "Material", "Reflectance", "Transparency", "Size", "CFrame", "Massless",
    "CustomPhysicalProperties", "CollisionGroup", "TopSurface", "BottomSurface",
    "LeftSurface", "RightSurface", "FrontSurface", "BackSurface"
}

local guiObject = {
    "AnchorPoint", "AutomaticSize", "BackgroundColor3", "BackgroundTransparency", "BorderColor3",
    "BorderMode", "BorderSizePixel", "LayoutOrder", "Position", "Rotation", "Size",
    "SizeConstraint", "Visible", "ZIndex"
}

local textObject = {
    "Font", "FontFace", "LineHeight", "MaxVisibleGraphemes", "RichText", "Text", "TextColor3",
    "TextScaled", "TextSize", "TextStrokeColor3", "TextStrokeTransparency", "TextTransparency",
    "TextTruncate", "TextWrapped", "TextXAlignment", "TextYAlignment"
}

local imageObject = {
    "ImageColor3", "ImageRectOffset", "ImageRectSize", "ImageTransparency",
    "ScaleType", "SliceCenter", "TileSize"
}

local attachment = { "CFrame", "Position", "Orientation", "Axis", "SecondaryAxis", "Visible" }

local contentPropertiesByClass = {
    Animation = { AnimationId = "animation" },
    Beam = { Texture = "image" },
    Decal = { Texture = "image" },
    ImageButton = { Image = "image" },
    ImageLabel = { Image = "image" },
    MeshPart = { MeshId = "mesh", TextureID = "image" },
    ParticleEmitter = { Texture = "image" },
    ScrollingFrame = { BottomImage = "image", MidImage = "image", TopImage = "image" },
    Sound = { SoundId = "audio" },
    SpecialMesh = { MeshId = "mesh", TextureId = "image" },
    SurfaceAppearance = {
        ColorMap = "image",
        MetalnessMap = "image",
        NormalMap = "image",
        RoughnessMap = "image"
    },
    Texture = { Texture = "image" },
    Trail = { Texture = "image" }
}

local contentPropertyOrder = {
    Animation = { "AnimationId" },
    Beam = { "Texture" },
    Decal = { "Texture" },
    ImageButton = { "Image" },
    ImageLabel = { "Image" },
    MeshPart = { "MeshId", "TextureID" },
    ParticleEmitter = { "Texture" },
    ScrollingFrame = { "BottomImage", "MidImage", "TopImage" },
    Sound = { "SoundId" },
    SpecialMesh = { "MeshId", "TextureId" },
    SurfaceAppearance = { "ColorMap", "MetalnessMap", "NormalMap", "RoughnessMap" },
    Texture = { "Texture" },
    Trail = { "Texture" }
}

local byClass = {
    Folder = {},
    Configuration = {},
    Model = { "PrimaryPart", "WorldPivot" },
    Tool = { "CanBeDropped", "Enabled", "Grip", "GripForward", "GripPos", "GripRight", "GripUp", "ManualActivationOnly", "RequiresHandle", "ToolTip" },
    MeshPart = { "CollisionFidelity", "DoubleSided", "RenderFidelity" },
    SpecialMesh = { "MeshType", "Offset", "Scale", "VertexColor" },
    UnionOperation = { "UsePartColor" },
    SurfaceAppearance = { "AlphaMode", "Color" },
    Weld = { "Part0", "Part1", "C0", "C1" },
    ManualWeld = { "Part0", "Part1", "C0", "C1" },
    Motor = { "Part0", "Part1", "C0", "C1", "CurrentAngle", "DesiredAngle", "MaxVelocity" },
    Motor6D = { "Part0", "Part1", "C0", "C1", "Transform" },
    WeldConstraint = { "Part0", "Part1", "Enabled" },
    NoCollisionConstraint = { "Part0", "Part1", "Enabled" },
    Snap = { "Part0", "Part1", "C0", "C1" },
    Script = { "Source", "Enabled", "RunContext" },
    LocalScript = { "Source", "Enabled" },
    ModuleScript = { "Source" },
    AnimationController = {},
    Animator = {},
    Animation = {},
    BindableEvent = {},
    BindableFunction = {},
    RemoteEvent = {},
    RemoteFunction = {},
    Sound = { "Volume", "PlaybackSpeed", "Looped", "RollOffMode", "RollOffMinDistance", "RollOffMaxDistance", "EmitterSize", "Playing", "TimePosition" },
    ParticleEmitter = {
        "Acceleration", "Brightness", "Color", "Drag", "EmissionDirection", "Enabled", "Lifetime",
        "LightEmission", "LightInfluence", "LockedToPart", "Orientation", "Rate", "RotSpeed",
        "Rotation", "Size", "Speed", "SpreadAngle", "Transparency", "VelocityInheritance",
        "ZOffset"
    },
    PointLight = { "Brightness", "Color", "Enabled", "Range", "Shadows" },
    SpotLight = { "Angle", "Brightness", "Color", "Enabled", "Face", "Range", "Shadows" },
    SurfaceLight = { "Angle", "Brightness", "Color", "Enabled", "Face", "Range", "Shadows" },
    Beam = {
        "Attachment0", "Attachment1", "Brightness", "Color", "CurveSize0", "CurveSize1", "Enabled",
        "FaceCamera", "LightEmission", "LightInfluence", "Segments", "TextureLength",
        "TextureMode", "TextureSpeed", "Transparency", "Width0", "Width1", "ZOffset"
    },
    Trail = {
        "Attachment0", "Attachment1", "Color", "Enabled", "FaceCamera", "Lifetime", "LightEmission",
        "LightInfluence", "MaxLength", "MinLength", "TextureLength", "TextureMode",
        "Transparency", "WidthScale"
    },
    Decal = { "Color3", "Face", "Transparency" },
    Texture = { "Color3", "Face", "StudsPerTileU", "StudsPerTileV", "Transparency" },
    StringValue = { "Value" },
    NumberValue = { "Value" },
    BoolValue = { "Value" },
    IntValue = { "Value" },
    ObjectValue = { "Value" },
    CFrameValue = { "Value" },
    Vector3Value = { "Value" },
    Color3Value = { "Value" },
    BrickColorValue = { "Value" },
    ScreenGui = { "DisplayOrder", "Enabled", "IgnoreGuiInset", "ResetOnSpawn", "ScreenInsets", "ZIndexBehavior" },
    SurfaceGui = { "Adornee", "AlwaysOnTop", "Brightness", "CanvasSize", "Enabled", "Face", "LightInfluence", "PixelsPerStud", "SizingMode", "ZIndexBehavior" },
    BillboardGui = { "Adornee", "AlwaysOnTop", "Brightness", "Enabled", "ExtentsOffset", "ExtentsOffsetWorldSpace", "LightInfluence", "MaxDistance", "Size", "SizeOffset", "StudsOffset", "StudsOffsetWorldSpace" },
    Frame = {},
    TextLabel = {},
    TextButton = { "AutoButtonColor", "Modal", "Selected", "Style" },
    TextBox = { "ClearTextOnFocus", "CursorPosition", "MultiLine", "PlaceholderColor3", "PlaceholderText", "ShowNativeInput", "TextEditable" },
    ImageLabel = {},
    ImageButton = { "AutoButtonColor", "Modal", "Selected", "Style" },
    ScrollingFrame = { "AutomaticCanvasSize", "CanvasPosition", "CanvasSize", "ElasticBehavior", "HorizontalScrollBarInset", "ScrollBarImageColor3", "ScrollBarImageTransparency", "ScrollBarThickness", "ScrollingDirection", "ScrollingEnabled", "VerticalScrollBarInset", "VerticalScrollBarPosition" },
    UIListLayout = { "FillDirection", "HorizontalAlignment", "Padding", "SortOrder", "VerticalAlignment" },
    UIGridLayout = { "CellPadding", "CellSize", "FillDirection", "HorizontalAlignment", "SortOrder", "StartCorner", "VerticalAlignment" },
    UICorner = { "CornerRadius" },
    UIStroke = { "ApplyStrokeMode", "Color", "Enabled", "LineJoinMode", "Thickness", "Transparency" },
    UIGradient = { "Color", "Enabled", "Offset", "Rotation", "Transparency" },
    UIPadding = { "PaddingBottom", "PaddingLeft", "PaddingRight", "PaddingTop" },
    UIScale = { "Scale" },
    UISizeConstraint = { "MaxSize", "MinSize" },
    UIAspectRatioConstraint = { "AspectRatio", "AspectType", "DominantAxis" },
    HingeConstraint = { "ActuatorType", "AngularResponsiveness", "AngularSpeed", "Attachment0", "Attachment1", "Enabled", "LimitsEnabled", "LowerAngle", "MotorMaxAcceleration", "MotorMaxTorque", "Radius", "Restitution", "ServoMaxTorque", "TargetAngle", "UpperAngle" },
    BallSocketConstraint = { "Attachment0", "Attachment1", "Enabled", "LimitsEnabled", "MaxFrictionTorque", "Radius", "Restitution", "TwistLimitsEnabled", "TwistLowerAngle", "TwistUpperAngle", "UpperAngle" },
    RigidConstraint = { "Attachment0", "Attachment1", "Enabled", "Visible" },
    RodConstraint = { "Attachment0", "Attachment1", "Enabled", "Length", "LimitAngle0", "LimitAngle1", "LimitsEnabled", "Thickness", "Visible" },
    PrismaticConstraint = { "ActuatorType", "Attachment0", "Attachment1", "Enabled", "LimitsEnabled", "LowerLimit", "MotorMaxAcceleration", "MotorMaxForce", "Restitution", "ServoMaxForce", "Size", "Speed", "TargetPosition", "UpperLimit", "Velocity" },
    CylindricalConstraint = { "ActuatorType", "AngularActuatorType", "AngularLimitsEnabled", "AngularResponsiveness", "AngularSpeed", "Attachment0", "Attachment1", "Enabled", "LimitsEnabled", "LowerAngle", "LowerLimit", "MotorMaxAcceleration", "MotorMaxForce", "Restitution", "ServoMaxForce", "Speed", "TargetAngle", "TargetPosition", "UpperAngle", "UpperLimit" },
    UniversalConstraint = { "Attachment0", "Attachment1", "Enabled", "LimitsEnabled", "MaxAngle", "Restitution", "Radius" },
    AlignPosition = { "ApplyAtCenterOfMass", "Attachment0", "Attachment1", "Enabled", "ForceLimitMode", "MaxAxesForce", "MaxForce", "MaxVelocity", "Mode", "Position", "ReactionForceEnabled", "Responsiveness", "RigidityEnabled" },
    AlignOrientation = { "AlignType", "Attachment0", "Attachment1", "CFrame", "Enabled", "MaxAngularVelocity", "MaxTorque", "Mode", "PrimaryAxis", "PrimaryAxisOnly", "ReactionTorqueEnabled", "Responsiveness", "RigidityEnabled", "SecondaryAxis" },
    LinearVelocity = { "Attachment0", "Attachment1", "Enabled", "ForceLimitMode", "LineDirection", "LineVelocity", "MaxAxesForce", "MaxForce", "MaxPlanarAxesForce", "PlaneVelocity", "RelativeTo", "VectorVelocity", "VelocityConstraintMode" },
    AngularVelocity = { "AngularVelocity", "Attachment0", "Enabled", "MaxTorque", "ReactionTorqueEnabled", "RelativeTo" },
    VectorForce = { "ApplyAtCenterOfMass", "Attachment0", "Enabled", "Force", "RelativeTo" },
    Torque = { "Attachment0", "Enabled", "RelativeTo", "Torque" },
    LineForce = { "ApplyAtCenterOfMass", "Attachment0", "Attachment1", "Enabled", "InverseSquareLaw", "Magnitude", "MaxForce", "ReactionForceEnabled" },
    RopeConstraint = { "Attachment0", "Attachment1", "Enabled", "Length", "Restitution", "Thickness", "Visible" },
    SpringConstraint = { "Attachment0", "Attachment1", "Coils", "Damping", "Enabled", "FreeLength", "LimitsEnabled", "MaxForce", "MaxLength", "MinLength", "Radius", "Stiffness", "Thickness", "Visible" }
}

local function append(target, source)
    for _, value in ipairs(source) do
        table.insert(target, value)
    end
end

local function appendContentProperties(target, className)
    local names = contentPropertyOrder[className]
    if not names then
        return
    end
    for _, property in ipairs(names) do
        table.insert(target, property)
    end
end

function Allowlist.forInstance(instance)
    local props = {}
    append(props, common)

    if instance:IsA("BasePart") then
        append(props, basePart)
    end
    if instance:IsA("GuiObject") then
        append(props, guiObject)
    end
    if instance:IsA("TextLabel") or instance:IsA("TextButton") or instance:IsA("TextBox") then
        append(props, textObject)
    end
    if instance:IsA("ImageLabel") or instance:IsA("ImageButton") then
        append(props, imageObject)
    end
    if instance:IsA("Attachment") then
        append(props, attachment)
    end

    append(props, byClass[instance.ClassName] or {})
    appendContentProperties(props, instance.ClassName)
    return props
end

function Allowlist.contentPropertiesForClass(className)
    local source = contentPropertiesByClass[className]
    local copy = {}
    if source then
        for property, kind in pairs(source) do
            copy[property] = kind
        end
    end
    return copy
end

function Allowlist.contentKindForClass(className, property)
    local props = contentPropertiesByClass[className]
    return props and props[property] or nil
end

function Allowlist.contentKindFor(instance, property)
    if not instance then
        return nil
    end
    return Allowlist.contentKindForClass(instance.ClassName, property)
end

return Allowlist
