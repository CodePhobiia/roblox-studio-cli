use crate::error::AppResult;
use crate::protocol::messages::{DeserializeRequest, SerializeRequest};
use serde_json::Value;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn snapshot_run(
    port: u16,
    studio: Option<String>,
    path: String,
    out: PathBuf,
    json: bool,
) -> AppResult<()> {
    let blob: Value = crate::cli::request::post(
        port,
        "transaction snapshot",
        "/serialize",
        &SerializeRequest {
            studio: studio.clone(),
            path: path.clone(),
        },
        180,
    )?;
    let snapshot = serde_json::json!({
        "kind": "rsTransactionSnapshot",
        "sourceStudio": studio,
        "sourcePath": path,
        "generatedUnixSeconds": now_unix_seconds(),
        "blob": blob
    });
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out, serde_json::to_string_pretty(&snapshot)?)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
    } else {
        println!("Snapshot written to {}", out.display());
    }
    Ok(())
}

pub fn restore_run(
    port: u16,
    studio: Option<String>,
    file: PathBuf,
    parent_path: String,
    if_exists: String,
    json: bool,
) -> AppResult<()> {
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&file)?)?;
    let blob = value.get("blob").cloned().unwrap_or(value);
    let response: Value = crate::cli::request::post(
        port,
        "transaction restore",
        "/deserialize",
        &DeserializeRequest {
            studio,
            parent_path,
            blob,
            conflict_mode: Some(if_exists),
            dry_run: false,
            rollback_on_error: true,
            package_id: None,
        },
        240,
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "Restored snapshot root at {}",
            response["rootPath"].as_str().unwrap_or("<unknown>")
        );
    }
    Ok(())
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
