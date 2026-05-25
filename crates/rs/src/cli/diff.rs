use crate::error::{AppError, AppResult};
use crate::protocol::messages::{ReadRequest, SnapshotDuplicateName};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiffChange {
    kind: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FixPlan {
    safe_to_apply: bool,
    operations: Vec<FixPlanOperation>,
    conflicts: Vec<String>,
    changes: Vec<DiffChange>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FixPlanOperation {
    action: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    property: Option<String>,
    reason: String,
}

#[derive(Debug, Clone)]
struct Node {
    class_name: String,
    properties: BTreeMap<String, Value>,
    attributes: BTreeMap<String, Value>,
    tags: Vec<String>,
}

#[derive(Debug, Clone)]
struct Tree {
    nodes: BTreeMap<String, Node>,
    duplicate_names: Vec<SnapshotDuplicateName>,
}

#[allow(clippy::too_many_arguments)]
pub fn run(
    port: u16,
    studio: Option<String>,
    path: Option<String>,
    export: Option<PathBuf>,
    against_studio: Option<String>,
    against_path: Option<String>,
    against_export: Option<PathBuf>,
    depth: u32,
    json: bool,
    ignore_scripts: bool,
    ignore_assets: bool,
    fix_plan: bool,
) -> AppResult<()> {
    let left = load_source(port, studio, path, export, depth)?;
    let right = load_source(port, against_studio, against_path, against_export, depth)?;
    let changes = diff_trees(&left, &right, ignore_scripts, ignore_assets);
    if fix_plan {
        let plan = build_fix_plan(changes, &left, &right);
        if json {
            println!("{}", serde_json::to_string_pretty(&plan)?);
        } else {
            println!("Fix plan: {} operation(s)", plan.operations.len());
            for operation in &plan.operations {
                match &operation.property {
                    Some(property) => {
                        println!(
                            "{} {}.{} - {}",
                            operation.action, operation.path, property, operation.reason
                        )
                    }
                    None => println!(
                        "{} {} - {}",
                        operation.action, operation.path, operation.reason
                    ),
                }
            }
            if !plan.conflicts.is_empty() {
                println!("Conflicts:");
                for conflict in &plan.conflicts {
                    println!("  - {conflict}");
                }
            }
            println!("Safe to apply: {}", plan.safe_to_apply);
        }
    } else if json {
        println!("{}", serde_json::to_string_pretty(&changes)?);
    } else {
        for change in &changes {
            let code = match change.kind.as_str() {
                "added" => "A",
                "deleted" => "D",
                "modified" => "M",
                "reference" => "R",
                _ => "?",
            };
            match &change.property {
                Some(property) if change.path == "." => println!("{code} {property}"),
                Some(property) => println!("{code} {}.{property}", change.path),
                None => println!("{code} {}", change.path),
            }
        }
        println!("{} change(s)", changes.len());
        let ambiguous = left.duplicate_names.len() + right.duplicate_names.len();
        if ambiguous > 0 {
            println!("Warnings: {ambiguous} duplicate-name path match risk(s)");
        }
    }
    Ok(())
}

fn build_fix_plan(changes: Vec<DiffChange>, left: &Tree, right: &Tree) -> FixPlan {
    let mut operations = Vec::new();
    let mut conflicts = Vec::new();
    for change in &changes {
        match change.kind.as_str() {
            "added" => operations.push(FixPlanOperation {
                action: "create".into(),
                path: change.path.clone(),
                property: None,
                reason: "right side has an instance missing from the left side".into(),
            }),
            "deleted" => operations.push(FixPlanOperation {
                action: "delete".into(),
                path: change.path.clone(),
                property: None,
                reason: "left side has an instance missing from the right side".into(),
            }),
            "modified" | "reference" => operations.push(FixPlanOperation {
                action: "set-property".into(),
                path: change.path.clone(),
                property: change.property.clone(),
                reason: if change.kind == "reference" {
                    "object reference topology differs".into()
                } else {
                    "property, attribute, tag, or class differs".into()
                },
            }),
            _ => conflicts.push(format!("unknown change kind at {}", change.path)),
        }
    }
    for duplicate in left
        .duplicate_names
        .iter()
        .chain(right.duplicate_names.iter())
    {
        conflicts.push(format!(
            "duplicate sibling name '{}' under {} makes path matching ambiguous",
            duplicate.name, duplicate.parent_path
        ));
    }
    FixPlan {
        safe_to_apply: conflicts.is_empty(),
        operations,
        conflicts,
        changes,
    }
}

fn load_source(
    port: u16,
    studio: Option<String>,
    path: Option<String>,
    export: Option<PathBuf>,
    depth: u32,
) -> AppResult<Tree> {
    if let Some(export) = export {
        return load_export(&export);
    }
    let path =
        path.ok_or_else(|| AppError::Other("--path is required for Studio diff sources".into()))?;
    let value: Value = crate::cli::request::post(
        port,
        "read",
        "/read",
        &ReadRequest {
            studio,
            path,
            depth,
        },
        75,
    )?;
    tree_from_read(value)
}

fn tree_from_read(value: Value) -> AppResult<Tree> {
    let root_path = value
        .get("fullPath")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Other("read response missing fullPath".into()))?
        .to_string();
    let mut nodes = BTreeMap::new();
    let mut duplicate_names = Vec::new();
    flatten_read_node(&value, &root_path, &mut nodes, &mut duplicate_names)?;
    Ok(Tree {
        nodes,
        duplicate_names,
    })
}

fn flatten_read_node(
    node: &Value,
    root_path: &str,
    out: &mut BTreeMap<String, Node>,
    duplicate_names: &mut Vec<SnapshotDuplicateName>,
) -> AppResult<()> {
    let full_path = node
        .get("fullPath")
        .and_then(Value::as_str)
        .unwrap_or(root_path);
    let key = relative_key(root_path, full_path);
    let mut child_counts = BTreeMap::<String, usize>::new();
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            if let Some(name) = child.get("name").and_then(Value::as_str) {
                *child_counts.entry(name.to_string()).or_default() += 1;
            }
        }
    }
    for (name, count) in child_counts {
        if count > 1 {
            duplicate_names.push(SnapshotDuplicateName {
                parent_path: full_path.to_string(),
                name,
                count,
            });
        }
    }
    out.insert(
        key,
        Node {
            class_name: node
                .get("className")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            properties: object_map(node.get("properties")),
            attributes: object_map(node.get("attributes")),
            tags: node
                .get("tags")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
        },
    );
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            flatten_read_node(child, root_path, out, duplicate_names)?;
        }
    }
    Ok(())
}

fn load_export(path: &Path) -> AppResult<Tree> {
    let root_path = find_export_root_path(path)?.ok_or_else(|| {
        AppError::Other(format!(
            "could not find export_manifest.json under {}",
            path.display()
        ))
    })?;
    let mut nodes = BTreeMap::new();
    let mut duplicate_names = Vec::new();
    for file in collect_instance_json(path)? {
        let value: Value = serde_json::from_str(&std::fs::read_to_string(&file)?)?;
        let full_path = value
            .get("fullPath")
            .and_then(Value::as_str)
            .unwrap_or(&root_path);
        nodes.insert(
            relative_key(&root_path, full_path),
            Node {
                class_name: value
                    .get("className")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                properties: object_map(value.get("properties")),
                attributes: object_map(value.get("attributes")),
                tags: value
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(ToOwned::to_owned)
                            .collect()
                    })
                    .unwrap_or_default(),
            },
        );
    }
    for duplicate in detect_duplicate_export_names(&nodes) {
        duplicate_names.push(duplicate);
    }
    Ok(Tree {
        nodes,
        duplicate_names,
    })
}

fn find_export_root_path(path: &Path) -> AppResult<Option<String>> {
    for file in collect_named(path, "export_manifest.json")? {
        let value: Value = serde_json::from_str(&std::fs::read_to_string(file)?)?;
        if let Some(root) = value.get("rootPath").and_then(Value::as_str) {
            return Ok(Some(root.to_string()));
        }
    }
    Ok(None)
}

fn collect_instance_json(path: &Path) -> AppResult<Vec<PathBuf>> {
    collect_named(path, "instance.json")
}

fn collect_named(path: &Path, name: &str) -> AppResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    fn walk(dir: &Path, name: &str, out: &mut Vec<PathBuf>) -> AppResult<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, name, out)?;
            } else if path.file_name().and_then(|value| value.to_str()) == Some(name) {
                out.push(path);
            }
        }
        Ok(())
    }
    walk(path, name, &mut out)?;
    out.sort();
    Ok(out)
}

fn diff_trees(
    left: &Tree,
    right: &Tree,
    ignore_scripts: bool,
    ignore_assets: bool,
) -> Vec<DiffChange> {
    let keys = left
        .nodes
        .keys()
        .chain(right.nodes.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();
    for key in keys {
        match (left.nodes.get(&key), right.nodes.get(&key)) {
            (Some(before), Some(after)) => diff_node(
                &key,
                before,
                after,
                ignore_scripts,
                ignore_assets,
                &mut changes,
            ),
            (Some(before), None) => changes.push(DiffChange {
                kind: "deleted".into(),
                path: key,
                property: None,
                before: Some(serde_json::json!(before.class_name)),
                after: None,
            }),
            (None, Some(after)) => changes.push(DiffChange {
                kind: "added".into(),
                path: key,
                property: None,
                before: None,
                after: Some(serde_json::json!(after.class_name)),
            }),
            (None, None) => {}
        }
    }
    changes
}

fn diff_node(
    path: &str,
    before: &Node,
    after: &Node,
    ignore_scripts: bool,
    ignore_assets: bool,
    changes: &mut Vec<DiffChange>,
) {
    if before.class_name != after.class_name {
        changes.push(DiffChange {
            kind: "modified".into(),
            path: path.to_string(),
            property: Some("ClassName".into()),
            before: Some(Value::String(before.class_name.clone())),
            after: Some(Value::String(after.class_name.clone())),
        });
    }
    diff_maps(
        path,
        "properties",
        &before.properties,
        &after.properties,
        ignore_scripts,
        ignore_assets,
        changes,
    );
    diff_maps(
        path,
        "attributes",
        &before.attributes,
        &after.attributes,
        false,
        false,
        changes,
    );
    if before.tags != after.tags {
        changes.push(DiffChange {
            kind: "modified".into(),
            path: path.to_string(),
            property: Some("tags".into()),
            before: Some(serde_json::json!(before.tags)),
            after: Some(serde_json::json!(after.tags)),
        });
    }
}

fn diff_maps(
    path: &str,
    scope: &str,
    before: &BTreeMap<String, Value>,
    after: &BTreeMap<String, Value>,
    ignore_scripts: bool,
    ignore_assets: bool,
    changes: &mut Vec<DiffChange>,
) {
    let keys = before
        .keys()
        .chain(after.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for key in keys {
        if ignore_scripts && key == "Source" {
            continue;
        }
        if ignore_assets && is_asset_property(&key) {
            continue;
        }
        let a = before.get(&key);
        let b = after.get(&key);
        if a != b {
            let kind = if a
                .and_then(Value::as_array)
                .and_then(|v| v.first())
                .and_then(Value::as_str)
                == Some("Ref")
                || b.and_then(Value::as_array)
                    .and_then(|v| v.first())
                    .and_then(Value::as_str)
                    == Some("Ref")
            {
                "reference"
            } else {
                "modified"
            };
            changes.push(DiffChange {
                kind: kind.into(),
                path: path.to_string(),
                property: Some(format!("{scope}.{key}")),
                before: a.cloned(),
                after: b.cloned(),
            });
        }
    }
}

fn is_asset_property(property: &str) -> bool {
    matches!(
        property,
        "AnimationId" | "Image" | "MeshId" | "SoundId" | "Texture" | "TextureID" | "TextureId"
    )
}

fn object_map(value: Option<&Value>) -> BTreeMap<String, Value> {
    value
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn relative_key(root_path: &str, full_path: &str) -> String {
    if full_path == root_path {
        ".".to_string()
    } else {
        full_path
            .strip_prefix(root_path)
            .and_then(|value| value.strip_prefix('.'))
            .unwrap_or(full_path)
            .to_string()
    }
}

fn detect_duplicate_export_names(nodes: &BTreeMap<String, Node>) -> Vec<SnapshotDuplicateName> {
    let mut by_parent = BTreeMap::<String, BTreeMap<String, usize>>::new();
    for key in nodes.keys() {
        if key == "." {
            continue;
        }
        let (parent, name) = key
            .rsplit_once('.')
            .map(|(parent, name)| (parent.to_string(), name.to_string()))
            .unwrap_or_else(|| (".".to_string(), key.clone()));
        *by_parent
            .entry(parent)
            .or_default()
            .entry(name)
            .or_default() += 1;
    }
    let mut out = Vec::new();
    for (parent_path, names) in by_parent {
        for (name, count) in names {
            if count > 1 {
                out.push(SnapshotDuplicateName {
                    parent_path: parent_path.clone(),
                    name,
                    count,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::relative_key;

    #[test]
    fn relative_key_strips_root() {
        assert_eq!(relative_key("Workspace.Tool", "Workspace.Tool"), ".");
        assert_eq!(
            relative_key("Workspace.Tool", "Workspace.Tool.Handle"),
            "Handle"
        );
    }
}
