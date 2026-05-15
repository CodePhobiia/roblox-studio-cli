use crate::bridge::auto_spawn::ensure_bridge_running;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{Envelope, ExportFile, ExportRequest, ExportResponse};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

pub fn run(
    port: u16,
    studio: Option<String>,
    path: String,
    out: PathBuf,
    depth: Option<u32>,
    overwrite: bool,
) -> AppResult<()> {
    ensure_bridge_running(port)?;
    let url = format!("http://127.0.0.1:{port}/export");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()?;
    let resp = client
        .post(&url)
        .json(&ExportRequest {
            studio,
            path,
            depth,
        })
        .send()
        .map_err(|source| AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?;
    let env: Envelope<ExportResponse> = resp.json()?;
    if !env.ok {
        return Err(crate::cli::envelope_error("export", env.error, env.code));
    }

    let response = env
        .data
        .ok_or_else(|| AppError::Other("export returned no data".into()))?;
    let mut counts = BTreeMap::<String, usize>::new();
    fs::create_dir_all(&out)?;

    for file in &response.files {
        let target = safe_join(&out, &file.path)?;
        if target.exists() && !overwrite {
            return Err(AppError::Other(format!(
                "refusing to overwrite existing file: {} (pass --overwrite)",
                target.display()
            )));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        write_export_file(&target, file)?;
        *counts.entry(file.kind.clone()).or_default() += 1;
    }

    println!(
        "Exported {} files from {} to {}",
        response.files.len(),
        response.root_path,
        out.display()
    );
    if !counts.is_empty() {
        println!("Kinds:");
        for (kind, count) in counts {
            println!("  {kind}: {count}");
        }
    }
    if !response.warnings.is_empty() {
        println!("Warnings ({}):", response.warnings.len());
        for warning in response.warnings.iter().take(20) {
            println!("  - {warning}");
        }
        if response.warnings.len() > 20 {
            println!("  ... ({} more)", response.warnings.len() - 20);
        }
    }
    std::io::stdout().flush()?;
    Ok(())
}

fn write_export_file(target: &Path, file: &ExportFile) -> AppResult<()> {
    if let Some(content) = &file.content {
        fs::write(target, content)?;
        return Ok(());
    }
    if let Some(json) = &file.json {
        fs::write(target, serde_json::to_string_pretty(json)?)?;
        return Ok(());
    }
    fs::write(target, "")?;
    Ok(())
}

fn safe_join(base: &Path, relative: &str) -> AppResult<PathBuf> {
    let rel = Path::new(relative);
    if rel.is_absolute() {
        return Err(AppError::Other(format!(
            "export file path must be relative: {relative}"
        )));
    }

    let mut target = PathBuf::from(base);
    for component in rel.components() {
        match component {
            Component::Normal(part) => target.push(part),
            _ => {
                return Err(AppError::Other(format!(
                    "unsafe export file path component in: {relative}"
                )))
            }
        }
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::safe_join;
    use std::path::Path;

    #[test]
    fn safe_join_rejects_parent_traversal() {
        assert!(safe_join(Path::new("out"), "../bad.txt").is_err());
    }

    #[test]
    fn safe_join_accepts_nested_relative_paths() {
        let path = safe_join(Path::new("out"), "Root/Child/file.json").unwrap();
        assert!(path.ends_with("Root/Child/file.json"));
    }
}
