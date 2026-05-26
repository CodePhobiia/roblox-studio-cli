use crate::error::{AppError, AppResult};
use crate::protocol::messages::{
    DeserializeRequest, ExportFile, ExportRequest, ExportResponse, PackageUpdateRequest,
    SerializeRequest, ValidateRequest, ValidateResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageManifest {
    package_version: u32,
    #[serde(default = "default_package_id")]
    package_id: String,
    source_studio: Option<String>,
    source_path: String,
    generated_unix_seconds: u64,
    command_version: String,
    validation_summary: Option<Value>,
    file_count: usize,
}

pub struct PackageUpdateFlags {
    pub owned_only: bool,
    pub preserve_local: bool,
    pub replace_owned: bool,
    pub conflict_report: bool,
    pub dry_run: bool,
    pub force: bool,
    pub json: bool,
}

#[derive(Debug)]
struct PackagePreflight {
    manifest: PackageManifest,
    blob: Value,
}

pub fn export_run(
    port: u16,
    studio: Option<String>,
    path: String,
    out: PathBuf,
    depth: Option<u32>,
    overwrite: bool,
) -> AppResult<()> {
    if out.exists() && !overwrite {
        return Err(AppError::Other(format!(
            "package output already exists: {} (pass --overwrite)",
            out.display()
        )));
    }
    std::fs::create_dir_all(&out)?;

    let export: ExportResponse = crate::cli::request::post(
        port,
        "package export",
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
        "package serialize",
        "/serialize",
        &SerializeRequest {
            studio: studio.clone(),
            path: path.clone(),
        },
        180,
    )?;
    let validation = crate::cli::request::post::<_, ValidateResponse>(
        port,
        "package validate",
        "/validate",
        &ValidateRequest {
            studio: studio.clone(),
            path: path.clone(),
            rules: Vec::new(),
        },
        75,
    )
    .ok();

    let export_root = out.join("tree");
    std::fs::create_dir_all(&export_root)?;
    for file in &export.files {
        let target = safe_join(&export_root, &file.path)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_export_file(&target, file)?;
    }
    std::fs::write(
        out.join("transfer_blob.json"),
        serde_json::to_string_pretty(&blob)?,
    )?;
    let validation_value = validation.as_ref().map(serde_json::to_value).transpose()?;
    if let Some(validation_value) = &validation_value {
        std::fs::write(
            out.join("validation.json"),
            serde_json::to_string_pretty(validation_value)?,
        )?;
    }
    let manifest = PackageManifest {
        package_version: 1,
        package_id: format!("rspkg-{}", now_unix_seconds()),
        source_studio: studio,
        source_path: path,
        generated_unix_seconds: now_unix_seconds(),
        command_version: env!("CARGO_PKG_VERSION").to_string(),
        validation_summary: validation_value
            .as_ref()
            .and_then(|value| value.get("summary").cloned()),
        file_count: export.files.len() + 3,
    };
    std::fs::write(
        out.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    let checksums = checksums(&out)?;
    std::fs::write(
        out.join("checksums.json"),
        serde_json::to_string_pretty(&checksums)?,
    )?;
    println!("Packaged {} into {}", export.root_path, out.display());
    println!("Files: {}", manifest.file_count);
    if let Some(summary) = manifest.validation_summary {
        println!("Validation: {}", serde_json::to_string(&summary)?);
    }
    Ok(())
}

pub fn inspect_run(file: PathBuf, json: bool) -> AppResult<()> {
    let manifest_path = file.join("manifest.json");
    let manifest: PackageManifest =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    } else {
        println!("Package: {}", file.display());
        println!("Version: {}", manifest.package_version);
        println!("Source: {}", manifest.source_path);
        println!("Command version: {}", manifest.command_version);
        println!("Files: {}", manifest.file_count);
        if let Some(summary) = manifest.validation_summary {
            println!("Validation: {}", serde_json::to_string(&summary)?);
        }
    }
    Ok(())
}

pub fn import_run(
    port: u16,
    studio: Option<String>,
    file: PathBuf,
    parent_path: String,
    if_exists: String,
    dry_run: bool,
    rollback_on_error: bool,
    image_rehost: Option<crate::cli::rehost_images::ImageRehostOptions>,
    json: bool,
) -> AppResult<()> {
    let PackagePreflight { manifest, mut blob } =
        preflight_package_for_mutation(&file, Some(&if_exists))?;
    let image_rehost_report = if let Some(options) = image_rehost {
        Some(crate::cli::rehost_images::rehost_image_refs_in_blob(
            &mut blob, &options,
        )?)
    } else {
        None
    };
    let mut response: Value = crate::cli::request::post(
        port,
        "package import",
        "/deserialize",
        &DeserializeRequest {
            studio,
            parent_path,
            blob,
            conflict_mode: Some(if_exists),
            dry_run,
            rollback_on_error,
            package_id: Some(manifest.package_id),
            validate_rules: Vec::new(),
            fail_on_validation_failure: false,
            fail_on_external_refs: false,
        },
        210,
    )?;
    if let Some(report) = &image_rehost_report {
        if let Some(object) = response.as_object_mut() {
            object.insert("imageRehost".into(), serde_json::to_value(report)?);
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "Imported package root at {}",
            response["rootPath"].as_str().unwrap_or("<unknown>")
        );
        if let Some(report) = &image_rehost_report {
            crate::cli::rehost_images::print_rehost_summary(report);
        }
        if let Some(warnings) = response.get("warnings").and_then(Value::as_array) {
            if !warnings.is_empty() {
                println!("Warnings ({}):", warnings.len());
                for warning in warnings.iter().take(20) {
                    println!("  - {}", warning.as_str().unwrap_or("<non-string warning>"));
                }
            }
        }
    }
    Ok(())
}

pub fn update_run(
    port: u16,
    studio: Option<String>,
    file: PathBuf,
    parent_path: String,
    flags: PackageUpdateFlags,
) -> AppResult<()> {
    let modes = [
        flags.owned_only,
        flags.preserve_local,
        flags.replace_owned,
        flags.conflict_report,
    ]
    .into_iter()
    .filter(|value| *value)
    .count();
    if modes > 1 {
        return Err(AppError::Other(
            "choose only one package update mode: --owned-only, --preserve-local, --replace-owned, or --conflict-report".into(),
        ));
    }
    let mode = if flags.conflict_report {
        "conflict-report"
    } else if flags.replace_owned {
        "replace-owned"
    } else if flags.preserve_local {
        "preserve-local"
    } else {
        "owned-only"
    };
    let PackagePreflight { manifest, blob } = preflight_package_for_mutation(&file, None)?;
    let response: Value = crate::cli::request::post(
        port,
        "package update",
        "/package-update",
        &PackageUpdateRequest {
            studio,
            parent_path,
            blob,
            package_id: manifest.package_id,
            mode: mode.to_string(),
            dry_run: flags.dry_run || flags.conflict_report,
            force: flags.force,
        },
        260,
    )?;
    if flags.json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        let verb = if flags.dry_run || flags.conflict_report {
            "Planned"
        } else {
            "Updated"
        };
        println!(
            "{verb} package update at {}",
            response["rootPath"].as_str().unwrap_or("<unknown>")
        );
        println!(
            "Mode: {}  Replaced: {}  Preserved: {}  Created: {}  Refused: {}",
            response["mode"].as_str().unwrap_or(mode),
            response["replaced"].as_u64().unwrap_or(0),
            response["preserved"].as_u64().unwrap_or(0),
            response["created"].as_u64().unwrap_or(0),
            response["refused"].as_u64().unwrap_or(0)
        );
        if let Some(warnings) = response.get("warnings").and_then(Value::as_array) {
            if !warnings.is_empty() {
                println!("Warnings:");
                for warning in warnings.iter().take(25) {
                    println!("  - {}", warning.as_str().unwrap_or("<warning>"));
                }
            }
        }
        print_json_string_list("Changed paths", response.get("changedPaths"), 20);
        print_json_string_list("Refused paths", response.get("refusedPaths"), 20);
    }
    Ok(())
}

pub fn verify_run(
    port: u16,
    studio: Option<String>,
    file: PathBuf,
    parent_path: Option<String>,
    if_exists: String,
    json: bool,
) -> AppResult<()> {
    validate_package_conflict_mode(&if_exists)?;
    let manifest = read_manifest_strict(&file)?;
    let blob = read_transfer_blob(&file)?;
    let checksum_report = verify_checksums(&file)?;
    let asset_refs = count_asset_refs(&file)?;
    let conflict_plan = if let (Some(studio), Some(parent_path)) = (studio, parent_path) {
        Some(crate::cli::request::post::<_, Value>(
            port,
            "package verify dry-run",
            "/deserialize",
            &DeserializeRequest {
                studio: Some(studio),
                parent_path,
                blob,
                conflict_mode: Some(if_exists),
                dry_run: true,
                rollback_on_error: false,
                package_id: Some(manifest.package_id.clone()),
                validate_rules: Vec::new(),
                fail_on_validation_failure: false,
                fail_on_external_refs: false,
            },
            120,
        )?)
    } else {
        None
    };
    let report = serde_json::json!({
        "ok": checksum_report.is_clean(),
        "manifest": manifest,
        "checksums": checksum_report,
        "transferBlobReadable": true,
        "assetReferenceCount": asset_refs,
        "conflictPlan": conflict_plan
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Package: {}", file.display());
        println!("Manifest: ok");
        println!("Transfer blob: readable");
        println!("Asset references: {asset_refs}");
        println!(
            "Checksums: {} ok, {} missing, {} mismatched, {} unexpected",
            report["checksums"]["ok"].as_u64().unwrap_or(0),
            report["checksums"]["missing"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0),
            report["checksums"]["mismatched"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0),
            report["checksums"]["unexpected"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0)
        );
        if let Some(plan) = report.get("conflictPlan").filter(|value| !value.is_null()) {
            println!("Dry-run conflict plan: {}", serde_json::to_string(plan)?);
        }
    }
    Ok(())
}

pub fn verify_package_folder(file: &Path) -> AppResult<Value> {
    let manifest = read_manifest_strict(file)?;
    let _blob = read_transfer_blob(file)?;
    let checksum_report = verify_checksums(file)?;
    let asset_refs = count_asset_refs(file)?;
    Ok(serde_json::json!({
        "ok": checksum_report.is_clean(),
        "manifest": manifest,
        "checksums": checksum_report,
        "transferBlobReadable": true,
        "assetReferenceCount": asset_refs
    }))
}

fn preflight_package_for_mutation(
    file: &Path,
    conflict_mode: Option<&str>,
) -> AppResult<PackagePreflight> {
    if let Some(conflict_mode) = conflict_mode {
        validate_package_conflict_mode(conflict_mode)?;
    }
    let manifest = read_manifest_strict(file)?;
    let blob = read_transfer_blob(file)?;
    let checksum_report = verify_checksums(file)?;
    if !checksum_report.is_clean() {
        return Err(AppError::Other(format!(
            "package checksum preflight failed: {}",
            checksum_failure_summary(&checksum_report)
        )));
    }
    Ok(PackagePreflight { manifest, blob })
}

fn validate_package_conflict_mode(mode: &str) -> AppResult<()> {
    match mode {
        "fail" | "replace" | "merge" | "rename" => Ok(()),
        other => Err(AppError::Other(format!(
            "unknown package conflict mode: {other}"
        ))),
    }
}

fn read_transfer_blob(file: &Path) -> AppResult<Value> {
    let blob_path = file.join("transfer_blob.json");
    let blob: Value = serde_json::from_str(&std::fs::read_to_string(&blob_path)?)?;
    validate_transfer_blob(&blob)?;
    Ok(blob)
}

fn validate_transfer_blob(blob: &Value) -> AppResult<()> {
    let version = blob
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| AppError::Other("transfer_blob.json missing numeric version".into()))?;
    if version != 1 {
        return Err(AppError::Other(format!(
            "unsupported transfer_blob.json version {version}; expected 1"
        )));
    }
    let root = blob
        .get("root")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Other("transfer_blob.json missing root id".into()))?;
    let instances = blob
        .get("instances")
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::Other("transfer_blob.json missing instances map".into()))?;
    if !instances.contains_key(root) {
        return Err(AppError::Other(format!(
            "transfer_blob.json root id '{root}' is not present in instances"
        )));
    }
    for (id, spec) in instances {
        let object = spec.as_object().ok_or_else(|| {
            AppError::Other(format!(
                "transfer_blob.json instance '{id}' is not an object"
            ))
        })?;
        if object
            .get("className")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .is_none()
        {
            return Err(AppError::Other(format!(
                "transfer_blob.json instance '{id}' missing className"
            )));
        }
        if let Some(properties) = object.get("properties") {
            if !properties.is_object() {
                return Err(AppError::Other(format!(
                    "transfer_blob.json instance '{id}' properties must be an object"
                )));
            }
        }
        if let Some(parent) = object.get("parent") {
            if !parent.is_null() && !parent.is_string() {
                return Err(AppError::Other(format!(
                    "transfer_blob.json instance '{id}' parent must be a string or null"
                )));
            }
        }
        if let Some(children) = object.get("children") {
            if !children.is_array() {
                return Err(AppError::Other(format!(
                    "transfer_blob.json instance '{id}' children must be an array"
                )));
            }
        }
    }
    Ok(())
}

fn checksum_failure_summary(report: &ChecksumReport) -> String {
    let mut parts = Vec::new();
    if !report.missing.is_empty() {
        parts.push(format!(
            "{} missing ({})",
            report.missing.len(),
            report
                .missing
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !report.mismatched.is_empty() {
        parts.push(format!(
            "{} mismatched ({})",
            report.mismatched.len(),
            report
                .mismatched
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !report.unexpected.is_empty() {
        parts.push(format!(
            "{} unexpected ({})",
            report.unexpected.len(),
            report
                .unexpected
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if parts.is_empty() {
        "ok".to_string()
    } else {
        parts.join("; ")
    }
}

pub fn pack_run(file: PathBuf, out: PathBuf) -> AppResult<()> {
    if !file.is_dir() {
        return Err(AppError::Other(format!(
            "package path is not a directory: {}",
            file.display()
        )));
    }
    let target = File::create(&out)?;
    let mut zip = zip::ZipWriter::new(target);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    add_zip_dir(&mut zip, &file, &file, options)?;
    zip.finish()
        .map_err(|err| AppError::Other(format!("could not finish zip: {err}")))?;
    println!("Packed {} into {}", file.display(), out.display());
    Ok(())
}

pub fn unpack_run(file: PathBuf, out: PathBuf, overwrite: bool) -> AppResult<()> {
    if out.exists() && !overwrite {
        return Err(AppError::Other(format!(
            "output folder already exists: {} (pass --overwrite)",
            out.display()
        )));
    }
    std::fs::create_dir_all(&out)?;
    let input = File::open(&file)?;
    let mut archive = zip::ZipArchive::new(input)
        .map_err(|err| AppError::Other(format!("could not read zip archive: {err}")))?;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| AppError::Other(format!("could not read zip entry: {err}")))?;
        let Some(enclosed) = entry.enclosed_name().map(PathBuf::from) else {
            return Err(AppError::Other(format!(
                "zip entry has unsafe path: {}",
                entry.name()
            )));
        };
        let target = out.join(enclosed);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut output = File::create(&target)?;
            std::io::copy(&mut entry, &mut output)?;
        }
    }
    println!("Unpacked {} into {}", file.display(), out.display());
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

fn print_json_string_list(label: &str, value: Option<&Value>, limit: usize) {
    let Some(items) = value.and_then(Value::as_array) else {
        return;
    };
    if items.is_empty() {
        return;
    }
    println!("{label} ({}):", items.len());
    for item in items.iter().take(limit) {
        println!("  - {}", item.as_str().unwrap_or("<non-string path>"));
    }
    if items.len() > limit {
        println!("  - ... {} more", items.len() - limit);
    }
}

fn safe_join(base: &Path, relative: &str) -> AppResult<PathBuf> {
    let rel = Path::new(relative);
    if rel.is_absolute() {
        return Err(AppError::Other(format!(
            "package file path must be relative: {relative}"
        )));
    }
    let mut target = PathBuf::from(base);
    for component in rel.components() {
        match component {
            Component::Normal(part) => target.push(part),
            _ => {
                return Err(AppError::Other(format!(
                    "unsafe package file path component in: {relative}"
                )))
            }
        }
    }
    Ok(target)
}

fn checksums(root: &Path) -> AppResult<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> AppResult<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else if path.file_name().and_then(|value| value.to_str()) != Some("checksums.json") {
                let bytes = std::fs::read(&path)?;
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, format!("{:016x}", fnv1a64(&bytes)));
            }
        }
        Ok(())
    }
    walk(root, root, &mut out)?;
    Ok(out)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChecksumReport {
    ok: usize,
    missing: Vec<String>,
    mismatched: Vec<String>,
    unexpected: Vec<String>,
}

fn read_manifest_strict(file: &Path) -> AppResult<PackageManifest> {
    read_manifest_with_policy(file, true)
}

fn read_manifest_with_policy(file: &Path, require_package_id: bool) -> AppResult<PackageManifest> {
    let manifest_path = file.join("manifest.json");
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    let package_version = raw
        .get("packageVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            AppError::Other(format!(
                "package manifest missing packageVersion: {}",
                manifest_path.display()
            ))
        })?;
    if package_version != 1 {
        return Err(AppError::Other(format!(
            "unsupported package manifest version {package_version}; expected 1"
        )));
    }
    let missing_package_id = match raw.get("packageId").and_then(Value::as_str) {
        Some(value) => value.trim().is_empty(),
        None => true,
    };
    if require_package_id && missing_package_id {
        return Err(AppError::Other(
            "package manifest packageId missing; refusing unsafe legacy package mutation".into(),
        ));
    }

    let manifest: PackageManifest = serde_json::from_value(raw)?;
    if require_package_id && manifest.package_id.trim().is_empty() {
        return Err(AppError::Other(
            "package manifest packageId missing; refusing unsafe legacy package mutation".into(),
        ));
    }
    Ok(manifest)
}

fn verify_checksums(root: &Path) -> AppResult<ChecksumReport> {
    let expected_path = root.join("checksums.json");
    let expected: BTreeMap<String, String> =
        serde_json::from_str(&std::fs::read_to_string(expected_path)?)?;
    let actual = checksums(root)?;
    let mut ok = 0usize;
    let mut missing = Vec::new();
    let mut mismatched = Vec::new();
    let mut expected_paths = BTreeSet::new();
    for (path, expected_hash) in expected {
        expected_paths.insert(path.clone());
        match actual.get(&path) {
            Some(actual_hash) if actual_hash == &expected_hash => ok += 1,
            Some(_) => mismatched.push(path),
            None => missing.push(path),
        }
    }
    let unexpected = actual
        .keys()
        .filter(|path| !expected_paths.contains(*path))
        .cloned()
        .collect();
    Ok(ChecksumReport {
        ok,
        missing,
        mismatched,
        unexpected,
    })
}

impl ChecksumReport {
    fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.mismatched.is_empty() && self.unexpected.is_empty()
    }
}

fn count_asset_refs(root: &Path) -> AppResult<usize> {
    let mut count = 0usize;
    fn walk(dir: &Path, count: &mut usize) -> AppResult<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, count)?;
            } else if path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.ends_with(".asset.json"))
            {
                *count += 1;
            }
        }
        Ok(())
    }
    walk(root, &mut count)?;
    Ok(count)
}

fn add_zip_dir<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    root: &Path,
    dir: &Path,
    options: zip::write::FileOptions,
) -> AppResult<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            if !name.is_empty() {
                zip.add_directory(format!("{name}/"), options)
                    .map_err(|err| AppError::Other(format!("could not add zip dir: {err}")))?;
            }
            add_zip_dir(zip, root, &path, options)?;
        } else {
            zip.start_file(name, options)
                .map_err(|err| AppError::Other(format!("could not add zip file: {err}")))?;
            let mut file = File::open(&path)?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            zip.write_all(&bytes)?;
        }
    }
    Ok(())
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn default_package_id() -> String {
    "rspkg-legacy".to_string()
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    fn package_dir(name: &str) -> PathBuf {
        let id = NEXT_DIR.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target-package-tests")
            .join(format!("{name}-{id}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_manifest(dir: &Path, include_package_id: bool) {
        let mut manifest = json!({
            "packageVersion": 1,
            "sourcePath": "Workspace.Model",
            "generatedUnixSeconds": 1,
            "commandVersion": "test",
            "fileCount": 2
        });
        if include_package_id {
            manifest["packageId"] = json!("rspkg-test");
        }
        std::fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    fn write_blob(dir: &Path, root_name: &str) {
        let blob = json!({
            "version": 1,
            "root": "i0",
            "instances": {
                "i0": {
                    "className": "Folder",
                    "parent": null,
                    "properties": { "Name": root_name },
                    "attributes": {},
                    "tags": [],
                    "children": []
                }
            },
            "warnings": []
        });
        std::fs::write(
            dir.join("transfer_blob.json"),
            serde_json::to_string_pretty(&blob).unwrap(),
        )
        .unwrap();
    }

    fn write_current_checksums(dir: &Path) {
        let map = checksums(dir).unwrap();
        std::fs::write(
            dir.join("checksums.json"),
            serde_json::to_string_pretty(&map).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn package_preflight_accepts_valid_package() {
        let dir = package_dir("valid");
        write_manifest(&dir, true);
        write_blob(&dir, "Model");
        write_current_checksums(&dir);

        let preflight = preflight_package_for_mutation(&dir, Some("fail")).unwrap();
        assert_eq!(preflight.manifest.package_id, "rspkg-test");
        assert_eq!(preflight.blob["root"], "i0");
    }

    #[test]
    fn package_preflight_rejects_legacy_manifest_without_package_id() {
        let dir = package_dir("legacy");
        write_manifest(&dir, false);
        write_blob(&dir, "Model");
        write_current_checksums(&dir);

        let err = preflight_package_for_mutation(&dir, Some("fail")).unwrap_err();
        assert!(
            err.to_string().contains("packageId missing"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn package_preflight_rejects_checksum_mismatch() {
        let dir = package_dir("checksum");
        write_manifest(&dir, true);
        write_blob(&dir, "Model");
        write_current_checksums(&dir);
        write_blob(&dir, "ChangedModel");

        let err = preflight_package_for_mutation(&dir, Some("fail")).unwrap_err();
        assert!(
            err.to_string().contains("checksum preflight failed"),
            "unexpected error: {err}"
        );
        assert!(
            err.to_string().contains("mismatched"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn package_preflight_rejects_unknown_conflict_mode_before_mutation() {
        let dir = package_dir("conflict-mode");
        let err = preflight_package_for_mutation(&dir, Some("allow")).unwrap_err();
        assert!(
            err.to_string().contains("unknown package conflict mode"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn verify_checksums_reports_unexpected_files() {
        let dir = package_dir("unexpected");
        write_manifest(&dir, true);
        write_blob(&dir, "Model");
        write_current_checksums(&dir);
        std::fs::write(dir.join("extra.txt"), "not in checksums").unwrap();

        let report = verify_checksums(&dir).unwrap();
        assert_eq!(report.unexpected, vec!["extra.txt".to_string()]);
        assert!(!report.is_clean());
    }

    #[test]
    fn transfer_blob_validation_requires_root_instance() {
        let err = validate_transfer_blob(&json!({
            "version": 1,
            "root": "i0",
            "instances": {}
        }))
        .unwrap_err();
        assert!(
            err.to_string().contains("root id 'i0'"),
            "unexpected error: {err}"
        );
    }
}
