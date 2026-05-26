use crate::error::{AppError, AppResult};
use crate::protocol::messages::{UpsertFileItem, UpsertFilesRequest, UpsertFilesResponse};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncManifest {
    targets: Vec<SyncTarget>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncTarget {
    folder: PathBuf,
    to: String,
    #[serde(default)]
    patterns: Vec<String>,
    kind: Option<String>,
    #[serde(rename = "sourceId")]
    source_id: Option<String>,
}

pub fn run(
    port: u16,
    studio: Option<String>,
    folder: Option<PathBuf>,
    parent_path: Option<String>,
    manifest: Option<PathBuf>,
    watch: bool,
    dry_run: bool,
    delete: bool,
    force: bool,
) -> AppResult<()> {
    let targets = load_targets(folder, parent_path, manifest)?;
    run_once(port, studio.clone(), &targets, dry_run, delete, force)?;
    if watch {
        println!("Watching for changes. Press Ctrl+C to stop.");
        let mut last = fingerprint_targets(&targets)?;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let next = fingerprint_targets(&targets)?;
            if next != last {
                run_once(port, studio.clone(), &targets, dry_run, delete, force)?;
                last = next;
            }
        }
    }
    Ok(())
}

fn load_targets(
    folder: Option<PathBuf>,
    parent_path: Option<String>,
    manifest: Option<PathBuf>,
) -> AppResult<Vec<SyncTarget>> {
    if let Some(manifest_path) = manifest {
        let root = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let manifest: SyncManifest =
            serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?;
        return Ok(manifest
            .targets
            .into_iter()
            .map(|mut target| {
                if target.folder.is_relative() {
                    target.folder = root.join(&target.folder);
                }
                target
            })
            .collect());
    }
    Ok(vec![SyncTarget {
        folder: folder
            .ok_or_else(|| AppError::Other("--folder or --manifest is required".into()))?,
        to: parent_path.unwrap_or_else(|| "Workspace".to_string()),
        patterns: Vec::new(),
        kind: None,
        source_id: None,
    }])
}

fn run_once(
    port: u16,
    studio: Option<String>,
    targets: &[SyncTarget],
    dry_run: bool,
    delete: bool,
    force: bool,
) -> AppResult<()> {
    for target in targets {
        let source_id = target_source_id(target);
        let files = collect_files(&target.folder, &target.patterns)?;
        let mut script_items = Vec::new();
        let mut png_files = Vec::new();
        let mut mesh_files = Vec::new();
        for file in files {
            if let Some(class_name) = script_class_for(&file) {
                script_items.push(script_item(&target.folder, &file, class_name)?);
            } else if has_ext(&file, &["png"]) {
                png_files.push(file);
            } else if has_ext(
                &file,
                &[
                    "obj", "stl", "gltf", "glb", "fbx", "dae", "ply", "abc", "usd", "blend",
                ],
            ) {
                mesh_files.push(file);
            }
        }

        if !script_items.is_empty() || delete {
            let response: UpsertFilesResponse = crate::cli::request::post(
                port,
                "sync-folder",
                "/upsert-files",
                &UpsertFilesRequest {
                    studio: studio.clone(),
                    parent_path: target.to.clone(),
                    dry_run,
                    delete,
                    force,
                    items: script_items,
                    source_id: Some(source_id.clone()),
                },
                150,
            )?;
            println!(
                "Synced scripts to {}: {} created, {} updated, {} unchanged, {} deleted",
                response.parent_path,
                response.created,
                response.updated,
                response.unchanged,
                response.deleted
            );
        }

        for file in png_files {
            crate::cli::import_image::run(
                port,
                studio.clone(),
                file.clone(),
                target.to.clone(),
                None,
                target.kind.clone().unwrap_or_else(|| "image".to_string()),
                None,
                "0,0".to_string(),
            )?;
        }
        for file in mesh_files {
            crate::cli::import_asset::run(
                port,
                studio.clone(),
                file,
                target.to.clone(),
                None,
                1.0,
                false,
                true,
                None,
            )?;
        }
    }
    Ok(())
}

fn collect_files(root: &Path, patterns: &[String]) -> AppResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    fn walk(dir: &Path, root: &Path, patterns: &[String], out: &mut Vec<PathBuf>) -> AppResult<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, patterns, out)?;
            } else if patterns.is_empty()
                || patterns
                    .iter()
                    .any(|pattern| matches_pattern(root, &path, pattern))
            {
                out.push(path);
            }
        }
        Ok(())
    }
    walk(root, root, patterns, &mut out)?;
    out.sort();
    Ok(out)
}

fn matches_pattern(root: &Path, path: &Path, pattern: &str) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let name = rel
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if pattern == "*" || pattern == "**/*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return name.ends_with(suffix);
    }
    name == pattern
}

fn script_class_for(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    if name.ends_with(".server.lua") {
        Some("Script")
    } else if name.ends_with(".client.lua") {
        Some("LocalScript")
    } else if name.ends_with(".module.lua") || name.ends_with(".lua") {
        Some("ModuleScript")
    } else {
        None
    }
}

fn script_item(root: &Path, file: &Path, class_name: &str) -> AppResult<UpsertFileItem> {
    let relative = file
        .strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/");
    crate::cli::export::validate_safe_relative_path(&relative, "sync-folder file path")?;
    let stem = file
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Script")
        .trim_end_matches(".server.lua")
        .trim_end_matches(".client.lua")
        .trim_end_matches(".module.lua")
        .trim_end_matches(".lua")
        .to_string();
    Ok(UpsertFileItem {
        path: relative,
        class_name: class_name.to_string(),
        name: stem,
        source: Some(std::fs::read_to_string(file)?),
        attributes: BTreeMap::new(),
    })
}

fn target_source_id(target: &SyncTarget) -> String {
    target.source_id.clone().unwrap_or_else(|| {
        let root = target
            .folder
            .canonicalize()
            .unwrap_or_else(|_| target.folder.clone())
            .to_string_lossy()
            .replace('\\', "/");
        stable_sync_source_id(&format!("{}|{}", root, target.to))
    })
}

pub(crate) fn stable_sync_source_id(seed: &str) -> String {
    format!("sync-{:016x}", fnv1a64(seed.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn has_ext(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| {
            extensions
                .iter()
                .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        })
}

fn fingerprint_targets(targets: &[SyncTarget]) -> AppResult<Vec<(PathBuf, u64, u64)>> {
    let mut rows = Vec::new();
    for target in targets {
        for file in collect_files(&target.folder, &target.patterns)? {
            let metadata = std::fs::metadata(&file)?;
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            rows.push((file, metadata.len(), modified));
        }
    }
    rows.sort();
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::{script_item, stable_sync_source_id, target_source_id, SyncTarget};
    use std::path::{Path, PathBuf};

    #[test]
    fn stable_sync_source_id_is_deterministic() {
        assert_eq!(
            stable_sync_source_id("folder|Workspace"),
            stable_sync_source_id("folder|Workspace")
        );
        assert_ne!(
            stable_sync_source_id("folder|Workspace"),
            stable_sync_source_id("other|Workspace")
        );
    }

    #[test]
    fn target_source_id_uses_manifest_value_when_supplied() {
        let target = SyncTarget {
            folder: PathBuf::from("src"),
            to: "ServerScriptService".to_string(),
            patterns: Vec::new(),
            kind: None,
            source_id: Some("sync-custom".to_string()),
        };
        assert_eq!(target_source_id(&target), "sync-custom");
    }

    #[test]
    fn script_item_rejects_unsafe_relative_path() {
        let root = Path::new("root");
        let bad_file = Path::new("root/Scripts/CON.server.lua");
        let result = script_item(root, bad_file, "Script");
        assert!(result.is_err());
    }
}
