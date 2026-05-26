use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PLUGIN_PROTOCOL_VERSION: u32 = 5;
pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope<T> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl<T> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            code: None,
        }
    }

    pub fn err(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error.into()),
            code: Some(code.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub id: String,
    pub name: String,
    pub place_file_path: Option<String>,
    #[serde(default)]
    pub protocol_version: Option<u32>,
    #[serde(default)]
    pub plugin_version: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
    pub session_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCommand {
    pub command_id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutopilotReviewRequest {
    pub studio: Option<String>,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioInfo {
    pub id: String,
    pub name: String,
    pub place_file_path: Option<String>,
    #[serde(default)]
    pub protocol_version: Option<u32>,
    #[serde(default)]
    pub plugin_version: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub last_heartbeat_ms_ago: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecRequest {
    pub studio: Option<String>,
    pub lua: String,
    #[serde(default)]
    pub allow_dangerous_exec: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadRequest {
    pub studio: Option<String>,
    pub path: String,
    #[serde(default = "default_depth")]
    pub depth: u32,
}

fn default_depth() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRequest {
    pub from_studio: String,
    pub from_path: String,
    pub to_studio: String,
    pub to_parent_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_mode: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub rollback_on_error: bool,
    #[serde(default)]
    pub allow_external_refs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAssetRequest {
    pub studio: Option<String>,
    pub parent_path: String,
    pub name: String,
    pub meshes: Vec<ImportMesh>,
    pub anchored: bool,
    pub weld: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMesh {
    pub name: String,
    pub vertices: Vec<[f32; 3]>,
    pub triangles: Vec<[usize; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[f32; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hierarchy_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_pivot: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAssetResponse {
    pub root_path: String,
    pub mesh_count: usize,
    pub part_count: usize,
    pub weld_count: usize,
    pub vertex_count: usize,
    pub triangle_count: usize,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportImageRequest {
    pub studio: Option<String>,
    pub parent_path: String,
    pub name: String,
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub ui_width: u32,
    pub ui_height: u32,
    pub position_x: i32,
    pub position_y: i32,
    pub pixels_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportImageResponse {
    pub root_path: String,
    pub gui_path: String,
    pub image_path: String,
    pub class_name: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub studio: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFile {
    pub path: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResponse {
    pub root_path: String,
    pub files: Vec<ExportFile>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializeRequest {
    pub studio: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeserializeRequest {
    pub studio: Option<String>,
    pub parent_path: String,
    pub blob: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_mode: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub rollback_on_error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validate_rules: Vec<String>,
    #[serde(default)]
    pub fail_on_validation_failure: bool,
    #[serde(default)]
    pub fail_on_external_refs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportUploadedRequest {
    pub studio: Option<String>,
    pub parent_path: String,
    pub kind: String,
    pub name: String,
    pub asset_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_y: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_speed: Option<f32>,
    #[serde(default)]
    pub looped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportUploadedResponse {
    pub root_path: String,
    pub instance_path: String,
    pub class_name: String,
    pub asset_id: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateRequest {
    pub studio: Option<String>,
    pub path: String,
    #[serde(default)]
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: String,
    pub rule: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationSummary {
    #[serde(default)]
    pub fail: usize,
    #[serde(default)]
    pub warn: usize,
    #[serde(default)]
    pub info: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateResponse {
    pub path: String,
    pub summary: ValidationSummary,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairToolRequest {
    pub studio: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
    pub dry_run: bool,
    pub replace_broken: bool,
    pub physics_fix: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collision: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub massless: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairToolResponse {
    pub path: String,
    pub dry_run: bool,
    #[serde(default)]
    pub fix_ids: Vec<String>,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    pub parts_found: usize,
    pub valid_joints_preserved: usize,
    pub broken_joints_found: usize,
    pub welds_created: usize,
    pub physics_properties_changed: usize,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRequest {
    pub studio: Option<String>,
    pub path: String,
    pub include_paths: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotAssetReference {
    pub path: String,
    pub property: String,
    pub asset_uri: String,
    pub asset_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSubtree {
    pub path: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDuplicateName {
    pub parent_path: String,
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotResponse {
    pub root_path: String,
    pub total_instances: usize,
    pub max_depth: usize,
    #[serde(default)]
    pub class_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub script_counts: BTreeMap<String, usize>,
    pub tool_count: usize,
    pub mesh_part_count: usize,
    pub ui_count: usize,
    pub remote_count: usize,
    #[serde(default)]
    pub asset_references: Vec<SnapshotAssetReference>,
    #[serde(default)]
    pub duplicate_sibling_names: Vec<SnapshotDuplicateName>,
    #[serde(default)]
    pub top_subtrees: Vec<SnapshotSubtree>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProperty {
    pub name: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstanceRequest {
    pub studio: Option<String>,
    pub parent_path: String,
    pub class_name: String,
    pub name: String,
    #[serde(default)]
    pub properties: Vec<CreateProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstanceResponse {
    pub path: String,
    pub class_name: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportUiPackElement {
    pub name: String,
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub size_scale_x: f32,
    pub size_offset_x: i32,
    pub size_scale_y: f32,
    pub size_offset_y: i32,
    pub position_scale_x: f32,
    pub position_offset_x: i32,
    pub position_scale_y: f32,
    pub position_offset_y: i32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_transparency: Option<f32>,
    pub pixels_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportUiPackRequest {
    pub studio: Option<String>,
    pub parent_path: String,
    pub name: String,
    pub elements: Vec<ImportUiPackElement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportUiPackResponse {
    pub gui_path: String,
    pub element_count: usize,
    #[serde(default)]
    pub element_paths: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAudioSound {
    pub name: String,
    pub asset_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_speed: Option<f32>,
    #[serde(default)]
    pub looped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAudioRequest {
    pub studio: Option<String>,
    pub parent_path: String,
    pub sounds: Vec<ImportAudioSound>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAudioResponse {
    pub parent_path: String,
    pub sound_count: usize,
    #[serde(default)]
    pub sound_paths: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertFileItem {
    pub path: String,
    pub class_name: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default)]
    pub attributes: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertFilesRequest {
    pub studio: Option<String>,
    pub parent_path: String,
    pub dry_run: bool,
    pub delete: bool,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub items: Vec<UpsertFileItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertFilesResponse {
    pub parent_path: String,
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub unchanged: usize,
    pub dry_run: bool,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanRequest {
    pub studio: Option<String>,
    pub root_path: String,
    pub plan: serde_json::Value,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub approved: bool,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub only: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyPlanResponse {
    pub root_path: String,
    pub dry_run: bool,
    pub applied: usize,
    pub skipped: usize,
    pub refused: usize,
    #[serde(default)]
    pub changed_paths: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageUpdateRequest {
    pub studio: Option<String>,
    pub parent_path: String,
    pub blob: serde_json::Value,
    pub package_id: String,
    pub mode: String,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRequest {
    pub studio: Option<String>,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepsRequest {
    pub studio: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyReference {
    pub path: String,
    pub class_name: String,
    pub property: String,
    pub kind: String,
    pub value: String,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub rule_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepsResponse {
    pub root_path: String,
    #[serde(default)]
    pub dependencies: Vec<DependencyReference>,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub remotes: Vec<String>,
    #[serde(default)]
    pub unowned_instances: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod diagnostic_contract_tests {
    use super::*;

    #[test]
    fn diagnostic_contract_dependency_reference_preserves_rule_ids() {
        let reference = DependencyReference {
            path: "Workspace.Tool.Handle".to_string(),
            class_name: "MeshPart".to_string(),
            property: "MeshId".to_string(),
            kind: "mesh".to_string(),
            value: "".to_string(),
            flags: vec!["missing".to_string()],
            rule_ids: vec!["asset.mesh.missing".to_string()],
        };

        let value = serde_json::to_value(&reference).unwrap();
        assert_eq!(value["ruleIds"][0], "asset.mesh.missing");
        let back: DependencyReference = serde_json::from_value(value).unwrap();
        assert_eq!(back.rule_ids, vec!["asset.mesh.missing".to_string()]);
    }

    #[test]
    fn diagnostic_contract_repair_response_allows_fix_ids_and_changed_paths() {
        let response = RepairToolResponse {
            path: "Workspace.Tool".to_string(),
            dry_run: true,
            fix_ids: vec!["fix.tool.weld-disconnected-parts".to_string()],
            changed_paths: vec!["Workspace.Tool.Part".to_string()],
            parts_found: 2,
            valid_joints_preserved: 0,
            broken_joints_found: 0,
            welds_created: 1,
            physics_properties_changed: 0,
            warnings: vec![],
        };

        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["fixIds"][0], "fix.tool.weld-disconnected-parts");
        assert_eq!(value["changedPaths"][0], "Workspace.Tool.Part");
        let back: RepairToolResponse = serde_json::from_value(value).unwrap();
        assert_eq!(back.fix_ids.len(), 1);
        assert_eq!(back.changed_paths.len(), 1);
    }
}
