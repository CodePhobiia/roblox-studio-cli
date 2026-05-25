use crate::error::{AppError, AppResult};
use crate::protocol::messages::{ExportFile, ExportRequest, ExportResponse, SerializeRequest};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
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
    for file in &export.files {
        let target = safe_join(&tree_root, &file.path)?;
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
        "rootPath": export.root_path,
        "generatedUnixSeconds": now_unix_seconds(),
        "fileCount": export.files.len(),
        "counts": counts,
        "transferBlob": "transfer_blob.json",
        "tree": "tree"
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

fn safe_join(base: &Path, relative: &str) -> AppResult<PathBuf> {
    let rel = Path::new(relative);
    if rel.is_absolute() {
        return Err(AppError::Other(format!(
            "sync-pull file path must be relative: {relative}"
        )));
    }
    let mut target = PathBuf::from(base);
    for component in rel.components() {
        match component {
            Component::Normal(part) => target.push(part),
            _ => {
                return Err(AppError::Other(format!(
                    "unsafe sync-pull path component in: {relative}"
                )))
            }
        }
    }
    Ok(target)
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
