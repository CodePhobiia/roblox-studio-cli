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
    "Image", "ImageColor3", "ImageRectOffset", "ImageRectSize", "ImageTransparency",
    "ScaleType", "SliceCenter", "TileSize"
}

local byClass = {
    Folder = {},
    Configuration = {},
    Model = { "PrimaryPart", "WorldPivot" },
    Tool = { "CanBeDropped", "Enabled", "Grip", "GripForward", "GripPos", "GripRight", "GripUp", "ManualActivationOnly", "RequiresHandle", "ToolTip" },
    MeshPart = { "MeshId", "TextureID", "CollisionFidelity", "DoubleSided", "RenderFidelity" },
    SpecialMesh = { "MeshId", "MeshType", "Offset", "Scale", "TextureId", "VertexColor" },
    UnionOperation = { "UsePartColor" },
    SurfaceAppearance = { "ColorMap", "NormalMap", "MetalnessMap", "RoughnessMap", "AlphaMode", "Color" },
    Weld = { "Part0", "Part1", "C0", "C1" },
    ManualWeld = { "Part0", "Part1", "C0", "C1" },
    Motor = { "Part0", "Part1", "C0", "C1", "CurrentAngle", "DesiredAngle", "MaxVelocity" },
    Motor6D = { "Part0", "Part1", "C0", "C1", "Transform" },
    WeldConstraint = { "Part0", "Part1", "Enabled" },
    NoCollisionConstraint = { "Part0", "Part1", "Enabled" },
    Snap = { "Part0", "Part1", "C0", "C1" },
    Attachment = { "CFrame", "Position", "Orientation", "Axis", "SecondaryAxis", "Visible" },
    Script = { "Source", "Enabled", "RunContext" },
    LocalScript = { "Source", "Enabled" },
    ModuleScript = { "Source" },
    AnimationController = {},
    Animator = {},
    Animation = { "AnimationId" },
    BindableEvent = {},
    BindableFunction = {},
    RemoteEvent = {},
    RemoteFunction = {},
    Sound = { "SoundId", "Volume", "PlaybackSpeed", "Looped", "RollOffMode", "RollOffMinDistance", "RollOffMaxDistance", "EmitterSize", "Playing", "TimePosition" },
    ParticleEmitter = {
        "Acceleration", "Brightness", "Color", "Drag", "EmissionDirection", "Enabled", "Lifetime",
        "LightEmission", "LightInfluence", "LockedToPart", "Orientation", "Rate", "RotSpeed",
        "Rotation", "Size", "Speed", "SpreadAngle", "Texture", "Transparency", "VelocityInheritance",
        "ZOffset"
    },
    PointLight = { "Brightness", "Color", "Enabled", "Range", "Shadows" },
    SpotLight = { "Angle", "Brightness", "Color", "Enabled", "Face", "Range", "Shadows" },
    SurfaceLight = { "Angle", "Brightness", "Color", "Enabled", "Face", "Range", "Shadows" },
    Beam = {
        "Attachment0", "Attachment1", "Brightness", "Color", "CurveSize0", "CurveSize1", "Enabled",
        "FaceCamera", "LightEmission", "LightInfluence", "Segments", "Texture", "TextureLength",
        "TextureMode", "TextureSpeed", "Transparency", "Width0", "Width1", "ZOffset"
    },
    Trail = {
        "Attachment0", "Attachment1", "Color", "Enabled", "FaceCamera", "Lifetime", "LightEmission",
        "LightInfluence", "MaxLength", "MinLength", "Texture", "TextureLength", "TextureMode",
        "Transparency", "WidthScale"
    },
    Decal = { "Color3", "Face", "Texture", "Transparency" },
    Texture = { "Color3", "Face", "StudsPerTileU", "StudsPerTileV", "Texture", "Transparency" },
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
    ScrollingFrame = { "AutomaticCanvasSize", "BottomImage", "CanvasPosition", "CanvasSize", "ElasticBehavior", "HorizontalScrollBarInset", "MidImage", "ScrollBarImageColor3", "ScrollBarImageTransparency", "ScrollBarThickness", "ScrollingDirection", "ScrollingEnabled", "TopImage", "VerticalScrollBarInset", "VerticalScrollBarPosition" },
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

    append(props, byClass[instance.ClassName] or {})
    return props
end

return Allowlist
