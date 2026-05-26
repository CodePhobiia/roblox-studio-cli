use crate::error::{AppError, AppResult};
use crate::protocol::messages::{ExportFile, ExportRequest, ExportResponse, SerializeRequest};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn run(
    port: u16,
    studio: Option<String>,
    path: String,
    out: PathBuf,
    depth: Option<u32>,
    overwrite: bool,
    json: bool,
) -> AppResult<()> {
    let export: ExportResponse = crate::cli::request::post(
        port,
        "sync pull export",
        "/export",
        &ExportRequest {
            studio: studio.clone(),
            path: path.clone(),
            depth,
        },
        180,
    )?;
    let blob: Value = crate::cli::request::post(
        port,
        "sync pull serialize",
        "/serialize",
        &SerializeRequest {
            studio: studio.clone(),
            path: path.clone(),
        },
        180,
    )?;

    std::fs::create_dir_all(&out)?;
    let tree_root = out.join("tree");
    std::fs::create_dir_all(&tree_root)?;
    let mut counts = BTreeMap::<String, usize>::new();
    let source_id = crate::cli::sync_folder::stable_sync_source_id(&format!(
        "{}|{}",
        studio.as_deref().unwrap_or("<default-studio>"),
        export.root_path
    ));
    let mut file_mappings = Vec::new();
    for file in &export.files {
        let target =
            crate::cli::export::safe_relative_join(&tree_root, &file.path, "sync-pull file path")?;
        if target.exists() && !overwrite {
            return Err(AppError::Other(format!(
                "refusing to overwrite existing file: {} (pass --overwrite)",
                target.display()
            )));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_export_file(&target, file)?;
        *counts.entry(file.kind.clone()).or_default() += 1;
        file_mappings.push(serde_json::json!({
            "path": file.path,
            "kind": file.kind,
            "className": file.json.as_ref().and_then(|json| json.get("className")).and_then(Value::as_str),
            "studioPath": file.json.as_ref().and_then(|json| json.get("fullPath")).and_then(Value::as_str),
            "sourceId": source_id.clone()
        }));
    }

    let blob_path = out.join("transfer_blob.json");
    if blob_path.exists() && !overwrite {
        return Err(AppError::Other(format!(
            "refusing to overwrite existing file: {} (pass --overwrite)",
            blob_path.display()
        )));
    }
    std::fs::write(&blob_path, serde_json::to_string_pretty(&blob)?)?;
    let manifest = serde_json::json!({
        "kind": "rsSyncPull",
        "sourceStudio": studio,
        "sourcePath": path,
        "rootPath": export.root_path.clone(),
        "sourceId": source_id.clone(),
        "generatedUnixSeconds": now_unix_seconds(),
        "fileCount": export.files.len(),
        "counts": counts,
        "transferBlob": "transfer_blob.json",
        "tree": "tree",
        "files": file_mappings,
        "syncTargets": [{
            "folder": "tree",
            "to": export.root_path.clone(),
            "sourceId": source_id
        }]
    });
    std::fs::write(
        out.join("sync_pull_manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    } else {
        println!(
            "Pulled {} files from {} into {}",
            export.files.len(),
            export.root_path,
            out.display()
        );
        println!("Transfer blob: {}", blob_path.display());
        if !export.warnings.is_empty() {
            println!("Warnings:");
            for warning in export.warnings.iter().take(20) {
                println!("  - {warning}");
            }
        }
    }
    Ok(())
}

fn write_export_file(target: &Path, file: &ExportFile) -> AppResult<()> {
    if let Some(content) = &file.content {
        std::fs::write(target, content)?;
    } else if let Some(json) = &file.json {
        std::fs::write(target, serde_json::to_string_pretty(json)?)?;
    } else {
        std::fs::write(target, "")?;
    }
    Ok(())
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
