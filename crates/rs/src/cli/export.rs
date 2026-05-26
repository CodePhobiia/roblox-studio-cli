use crate::error::{AppError, AppResult};
use crate::protocol::messages::{ExportFile, ExportRequest, ExportResponse};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub fn run(
    port: u16,
    studio: Option<String>,
    path: String,
    out: PathBuf,
    depth: Option<u32>,
    overwrite: bool,
) -> AppResult<()> {
    let response: ExportResponse = crate::cli::request::post(
        port,
        "export",
        "/export",
        &ExportRequest {
            studio,
            path,
            depth,
        },
        180,
    )?;
    let mut counts = BTreeMap::<String, usize>::new();
    fs::create_dir_all(&out)?;

    for file in &response.files {
        let target = safe_relative_join(&out, &file.path, "export file path")?;
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

pub(crate) fn safe_relative_join(base: &Path, relative: &str, context: &str) -> AppResult<PathBuf> {
    validate_safe_relative_path(relative, context)?;
    let rel = Path::new(relative);
    if rel.is_absolute() {
        return Err(AppError::Other(format!(
            "{context} must be relative: {relative}"
        )));
    }

    let mut target = PathBuf::from(base);
    for component in rel.components() {
        match component {
            Component::Normal(part) => target.push(part),
            _ => {
                return Err(AppError::Other(format!(
                    "unsafe {context} component in: {relative}"
                )))
            }
        }
    }
    Ok(target)
}

pub(crate) fn validate_safe_relative_path(relative: &str, context: &str) -> AppResult<()> {
    if relative.is_empty() {
        return Err(AppError::Other(format!("{context} must not be empty")));
    }
    if relative.starts_with('/') || relative.starts_with('\\') {
        return Err(AppError::Other(format!(
            "{context} must not start with a path separator: {relative}"
        )));
    }
    if relative.len() >= 2 && relative.as_bytes()[1] == b':' {
        return Err(AppError::Other(format!(
            "{context} must not use a drive or scheme prefix: {relative}"
        )));
    }

    let normalized = relative.replace('\\', "/");
    if normalized.ends_with('/') || normalized.contains("//") {
        return Err(AppError::Other(format!(
            "{context} contains an empty path segment: {relative}"
        )));
    }
    for segment in normalized.split('/') {
        validate_safe_segment(segment, context, relative)?;
    }
    Ok(())
}

fn validate_safe_segment(segment: &str, context: &str, original: &str) -> AppResult<()> {
    if segment.is_empty() {
        return Err(AppError::Other(format!(
            "{context} contains an empty path segment: {original}"
        )));
    }
    if matches!(segment, "." | "..") {
        return Err(AppError::Other(format!(
            "{context} contains an ambiguous path segment '{segment}': {original}"
        )));
    }
    if segment.ends_with('.') || segment.ends_with(' ') {
        return Err(AppError::Other(format!(
            "{context} segment must not end with '.' or space: {original}"
        )));
    }
    if segment
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Err(AppError::Other(format!(
            "{context} contains an invalid filename character: {original}"
        )));
    }
    let stem = segment
        .split_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(segment)
        .to_ascii_uppercase();
    if is_windows_reserved_name(&stem) {
        return Err(AppError::Other(format!(
            "{context} contains a reserved filename segment '{segment}': {original}"
        )));
    }
    Ok(())
}

fn is_windows_reserved_name(stem: &str) -> bool {
    matches!(stem, "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .is_some_and(|rest| matches!(rest, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
        || stem
            .strip_prefix("LPT")
            .is_some_and(|rest| matches!(rest, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
}

#[cfg(test)]
mod tests {
    use super::{safe_relative_join, validate_safe_relative_path};
    use std::path::Path;

    #[test]
    fn safe_relative_join_rejects_parent_traversal() {
        assert!(safe_relative_join(Path::new("out"), "../bad.txt", "test path").is_err());
    }

    #[test]
    fn safe_relative_join_accepts_nested_relative_paths() {
        let path =
            safe_relative_join(Path::new("out"), "Root/Child/file.json", "test path").unwrap();
        assert!(path.ends_with("Root/Child/file.json"));
    }

    #[test]
    fn safe_relative_path_rejects_reserved_and_ambiguous_segments() {
        for bad in [
            "",
            "/abs.txt",
            "C:/abs.txt",
            "Root//file.lua",
            "Root/./file.lua",
            "Root/CON.txt",
            "Root/name:.lua",
            "Root/trailing.",
        ] {
            assert!(
                validate_safe_relative_path(bad, "test path").is_err(),
                "{bad}"
            );
        }
    }
}
