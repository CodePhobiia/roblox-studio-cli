const PROPERTY_ALLOWLIST: &str = include_str!("../../../plugin/src/PropertyAllowlist.lua");

#[test]
fn property_allowlist_preserves_attachment_family_transforms() {
    assert!(
        PROPERTY_ALLOWLIST.contains(
            r#"local attachment = { "CFrame", "Position", "Orientation", "Axis", "SecondaryAxis", "Visible" }"#
        ),
        "Attachment-family transform properties must stay grouped together"
    );
    assert!(
        PROPERTY_ALLOWLIST.contains(r#"if instance:IsA("Attachment") then"#),
        "Bone inherits Attachment, so transform serialization must use IsA instead of exact ClassName"
    );
    assert!(
        !PROPERTY_ALLOWLIST.contains("Attachment = {"),
        "Attachment transform coverage must not depend on exact ClassName lookup"
    );
}
