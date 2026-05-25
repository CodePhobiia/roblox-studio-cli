use crate::error::{AppError, AppResult};
use crate::protocol::messages::{
    DeserializeRequest, ExportFile, ExportRequest, ExportResponse, PackageUpdateRequest,
    SerializeRequest, ValidateRequest, ValidateResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
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
    let manifest = read_manifest(&file).ok();
    let mut blob: Value =
        serde_json::from_str(&std::fs::read_to_string(file.join("transfer_blob.json"))?)?;
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
            package_id: manifest
                .as_ref()
                .map(|manifest| manifest.package_id.clone()),
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
    let manifest = read_manifest(&file)?;
    let blob: Value =
        serde_json::from_str(&std::fs::read_to_string(file.join("transfer_blob.json"))?)?;
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
    let manifest = read_manifest(&file)?;
    let blob_path = file.join("transfer_blob.json");
    let blob: Value = serde_json::from_str(&std::fs::read_to_string(&blob_path)?)?;
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
        "ok": checksum_report.missing.is_empty() && checksum_report.mismatched.is_empty(),
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
            "Checksums: {} ok, {} missing, {} mismatched",
            report["checksums"]["ok"].as_u64().unwrap_or(0),
            report["checksums"]["missing"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0),
            report["checksums"]["mismatched"]
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
    let manifest = read_manifest(file)?;
    let blob_path = file.join("transfer_blob.json");
    let _blob: Value = serde_json::from_str(&std::fs::read_to_string(&blob_path)?)?;
    let checksum_report = verify_checksums(file)?;
    let asset_refs = count_asset_refs(file)?;
    Ok(serde_json::json!({
        "ok": checksum_report.missing.is_empty() && checksum_report.mismatched.is_empty(),
        "manifest": manifest,
        "checksums": checksum_report,
        "transferBlobReadable": true,
        "assetReferenceCount": asset_refs
    }))
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
}

fn read_manifest(file: &Path) -> AppResult<PackageManifest> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        file.join("manifest.json"),
    )?)?)
}

fn verify_checksums(root: &Path) -> AppResult<ChecksumReport> {
    let expected_path = root.join("checksums.json");
    let expected: BTreeMap<String, String> =
        serde_json::from_str(&std::fs::read_to_string(expected_path)?)?;
    let actual = checksums(root)?;
    let mut ok = 0usize;
    let mut missing = Vec::new();
    let mut mismatched = Vec::new();
    for (path, expected_hash) in expected {
        match actual.get(&path) {
            Some(actual_hash) if actual_hash == &expected_hash => ok += 1,
            Some(_) => mismatched.push(path),
            None => missing.push(path),
        }
    }
    Ok(ChecksumReport {
        ok,
        missing,
        mismatched,
    })
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
