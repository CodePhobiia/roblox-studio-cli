use crate::protocol::messages::PLUGIN_PROTOCOL_VERSION;
use std::collections::BTreeSet;

const MAIN_SERVER: &str = include_str!("../../../plugin/src/Main.server.lua");
const PROPERTY_ALLOWLIST: &str = include_str!("../../../plugin/src/PropertyAllowlist.lua");
const EXPORTER: &str = include_str!("../../../plugin/src/Exporter.lua");
const DIAGNOSTICS: &str = include_str!("../../../plugin/src/Diagnostics.lua");
const DEPS_HANDLER: &str = include_str!("../../../plugin/src/Handlers/Deps.lua");
const SERVER: &str = include_str!("bridge/server.rs");
const AUTO_SPAWN: &str = include_str!("bridge/auto_spawn.rs");
const ROOT_README: &str = include_str!("../../../README.md");
const PLUGIN_README: &str = include_str!("../../../plugin/README.md");
const ARCHITECTURE_DOC: &str = include_str!("../../../docs/architecture.md");
const PROTOCOL_DOC: &str = include_str!("../../../docs/protocol.md");
const CI_WORKFLOW: &str = include_str!("../../../.github/workflows/ci.yml");

const CAPABILITY_ENDPOINTS: &[(&str, &str)] = &[
    ("exec", "/exec"),
    ("read", "/read"),
    ("export", "/export"),
    ("importAsset", "/import-asset"),
    ("importImage", "/import-image"),
    ("importUploaded", "/import-uploaded"),
    ("importUiPack", "/import-ui-pack"),
    ("importAudio", "/import-audio"),
    ("validate", "/validate"),
    ("repairTool", "/repair-tool"),
    ("snapshot", "/snapshot"),
    ("create", "/create"),
    ("upsertFiles", "/upsert-files"),
    ("applyPlan", "/apply-plan"),
    ("packageUpdate", "/package-update"),
    ("history", "/history"),
    ("deps", "/deps"),
    ("serialize", "/serialize"),
    ("deserialize", "/deserialize"),
    ("autopilotReview", "/autopilot-review"),
];

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

#[test]
fn content_property_matrix_covers_surface_appearance_maps() {
    for property in ["ColorMap", "MetalnessMap", "NormalMap", "RoughnessMap"] {
        let expected = format!(r#"{property} = "image""#);
        assert!(
            PROPERTY_ALLOWLIST.contains(&expected),
            "PropertyAllowlist content matrix must classify SurfaceAppearance.{property}"
        );
        assert!(
            DIAGNOSTICS.contains(&format!(r#"{property} = {{ kind = "image""#)),
            "Diagnostics asset matrix must classify SurfaceAppearance.{property} for deps/validate"
        );
    }
    assert!(
        DEPS_HANDLER.contains("Diagnostics.assetPropertiesFor(instance)"),
        "Deps handler must consume the shared Diagnostics asset matrix"
    );
    assert!(
        PROPERTY_ALLOWLIST.contains(r#"SurfaceAppearance = { "AlphaMode", "Color" }"#),
        "SurfaceAppearance non-content properties should remain allowlisted separately"
    );
}

#[test]
fn exporter_uses_shared_content_property_matrix() {
    assert!(
        EXPORTER.contains("local Allowlist = require(script.Parent.PropertyAllowlist)"),
        "Exporter must import the shared property/content support matrix"
    );
    assert!(
        EXPORTER.contains("Allowlist.contentKindForClass(metadata.className, property)"),
        "Exporter asset manifests must use the shared content property matrix"
    );
    assert!(
        !EXPORTER.contains("local ASSET_PROPERTIES"),
        "Exporter must not keep a separate asset-property map"
    );
}

#[test]
fn content_property_matrix_tracks_current_image_like_families() {
    for (class_name, properties) in [
        ("ImageLabel", &["Image"][..]),
        ("ImageButton", &["Image"][..]),
        (
            "ScrollingFrame",
            &["BottomImage", "MidImage", "TopImage"][..],
        ),
        ("MeshPart", &["TextureID"][..]),
        ("SpecialMesh", &["TextureId"][..]),
        ("Decal", &["Texture"][..]),
        ("Texture", &["Texture"][..]),
        ("ParticleEmitter", &["Texture"][..]),
        ("Trail", &["Texture"][..]),
        ("Beam", &["Texture"][..]),
    ] {
        let class_marker = format!("{class_name} = {{");
        assert!(
            PROPERTY_ALLOWLIST.contains(&class_marker),
            "PropertyAllowlist content matrix missing {class_name}"
        );
        for property in properties {
            assert!(
                PROPERTY_ALLOWLIST.contains(&format!(r#"{property} = "image""#))
                    || PROPERTY_ALLOWLIST.contains(&format!(r#"{property} = "mesh""#)),
                "PropertyAllowlist content matrix missing {class_name}.{property}"
            );
        }
    }
}

#[test]
fn plugin_capabilities_match_registered_handlers() {
    let capabilities = extract_luau_string_array(MAIN_SERVER, "CAPABILITIES");
    let dispatch_kinds = extract_dispatch_kinds(MAIN_SERVER);
    assert_unique("CAPABILITIES", &capabilities);
    assert_unique("Dispatch.register", &dispatch_kinds);

    assert_eq!(
        to_set(&capabilities),
        to_set(&dispatch_kinds),
        "Every advertised plugin capability must have a registered handler, and every handler must be advertised"
    );
}

#[test]
fn documented_capabilities_match_luau_and_bridge_routes() {
    let capabilities = extract_luau_string_array(MAIN_SERVER, "CAPABILITIES");
    let capability_set = to_set(&capabilities);
    assert_eq!(
        capability_set,
        CAPABILITY_ENDPOINTS
            .iter()
            .map(|(capability, _)| (*capability).to_string())
            .collect::<BTreeSet<_>>(),
        "Update CAPABILITY_ENDPOINTS when the plugin capability table changes"
    );

    for (capability, endpoint) in CAPABILITY_ENDPOINTS {
        assert!(
            PLUGIN_README.contains(&format!("- `{capability}` ->")),
            "plugin/README.md is missing capability `{capability}`"
        );
        assert!(
            PROTOCOL_DOC.contains(&format!("\"{capability}\"")),
            "docs/protocol.md registration example is missing capability `{capability}`"
        );
        assert!(
            SERVER.contains(&format!(".route(\"{endpoint}\"")),
            "bridge server is missing route `{endpoint}` for capability `{capability}`"
        );
        assert!(
            PROTOCOL_DOC.contains(&format!("| `POST` | `{endpoint}` |")),
            "docs/protocol.md is missing CLI endpoint `{endpoint}` for capability `{capability}`"
        );
    }
}

#[test]
fn protocol_version_docs_match_luau_and_rust_constants() {
    let luau_protocol = extract_luau_number(MAIN_SERVER, "PROTOCOL_VERSION");
    assert_eq!(
        luau_protocol, PLUGIN_PROTOCOL_VERSION,
        "Luau PROTOCOL_VERSION must match Rust PLUGIN_PROTOCOL_VERSION"
    );
    assert!(
        PROTOCOL_DOC.contains(&format!("\"protocolVersion\": {PLUGIN_PROTOCOL_VERSION}")),
        "docs/protocol.md registration example must use the current protocol version"
    );
    assert!(
        PLUGIN_README.contains(&format!(
            "Current plugin protocol: `{PLUGIN_PROTOCOL_VERSION}`"
        )),
        "plugin/README.md must name the current protocol version"
    );
}

#[test]
fn docs_match_auto_spawn_timeout_and_token_behavior() {
    let seconds = extract_auto_spawn_seconds();
    let seconds_text = format!("{seconds} seconds");
    assert!(
        ARCHITECTURE_DOC.contains(&seconds_text),
        "docs/architecture.md must document the current auto-spawn wait"
    );
    assert!(
        ROOT_README.contains(&seconds_text),
        "README.md must document the current auto-spawn wait"
    );

    for needle in [
        "RS_BRIDGE_TOKEN",
        "RS_BRIDGE_TOKEN_FILE",
        "x-rs-bridge-token",
        ".rs-bridge-token",
    ] {
        assert!(
            ROOT_README.contains(needle)
                && ARCHITECTURE_DOC.contains(needle)
                && PROTOCOL_DOC.contains(needle),
            "README.md, docs/architecture.md, and docs/protocol.md must document `{needle}`"
        );
    }
}

#[test]
fn readme_examples_are_labeled_by_side_effect() {
    for label in [
        "### Offline",
        "### Live Dry-Run",
        "### Live Apply",
        "### Cloud Side Effect",
    ] {
        assert!(
            ROOT_README.contains(label),
            "README.md must label examples with `{label}`"
        );
    }
}

#[test]
fn ci_runs_static_docs_capability_check() {
    assert!(
        CI_WORKFLOW.contains("cargo test -p rs plugin_static_tests"),
        ".github/workflows/ci.yml must run the static docs/capability drift tests explicitly"
    );
}

fn extract_luau_string_array(source: &str, name: &str) -> Vec<String> {
    let marker = format!("local {name} = {{");
    let start = source
        .find(&marker)
        .unwrap_or_else(|| panic!("missing Luau array `{name}`"))
        + marker.len();
    let body = &source[start..];
    let end = body
        .find("\n}")
        .or_else(|| body.find('}'))
        .unwrap_or_else(|| panic!("unterminated Luau array `{name}`"));

    body[..end]
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_end_matches(',');
            if trimmed.starts_with('"') && trimmed.ends_with('"') {
                Some(trimmed.trim_matches('"').to_string())
            } else {
                None
            }
        })
        .collect()
}

fn extract_dispatch_kinds(source: &str) -> Vec<String> {
    let marker = "Dispatch.register(\"";
    source
        .lines()
        .filter_map(|line| {
            let start = line.find(marker)? + marker.len();
            let rest = &line[start..];
            let end = rest.find('"')?;
            Some(rest[..end].to_string())
        })
        .collect()
}

fn extract_luau_number(source: &str, name: &str) -> u32 {
    let marker = format!("local {name} = ");
    source
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix(&marker)
                .and_then(|value| value.trim().parse::<u32>().ok())
        })
        .unwrap_or_else(|| panic!("missing Luau number `{name}`"))
}

fn extract_auto_spawn_seconds() -> u64 {
    let marker = "Duration::from_secs(";
    let start = AUTO_SPAWN
        .find(marker)
        .expect("auto-spawn deadline must use Duration::from_secs")
        + marker.len();
    let rest = &AUTO_SPAWN[start..];
    let end = rest.find(')').expect("auto-spawn duration call closes");
    rest[..end]
        .parse()
        .expect("auto-spawn duration must be a numeric literal")
}

fn assert_unique(name: &str, values: &[String]) {
    let set = to_set(values);
    assert_eq!(
        values.len(),
        set.len(),
        "{name} contains duplicate entries: {values:?}"
    );
}

fn to_set(values: &[String]) -> BTreeSet<String> {
    values.iter().cloned().collect()
}
