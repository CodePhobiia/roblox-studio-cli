use crate::error::{AppError, AppResult};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const ASSETS_URL: &str = "https://apis.roblox.com/assets/v1/assets";
const OPERATIONS_BASE_URL: &str = "https://apis.roblox.com/assets/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadKind {
    Image,
    Audio,
    Model,
    Mesh,
}

#[derive(Debug)]
pub struct UploadOptions {
    pub port: u16,
    pub studio: Option<String>,
    pub kind: UploadKind,
    pub file: PathBuf,
    pub creator_id: Option<u64>,
    pub creator_type: Option<String>,
    pub profile: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub api_key: Option<String>,
    pub wait: bool,
    pub wait_timeout_secs: u64,
    pub import_to: Option<String>,
    pub json_output: bool,
}

#[derive(Debug)]
pub(crate) struct UploadImageBytesOptions {
    pub bytes: Vec<u8>,
    pub filename: String,
    pub creator_id: Option<u64>,
    pub creator_type: Option<String>,
    pub profile: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub api_key: Option<String>,
    pub wait_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadSummary {
    asset_type: &'static str,
    file: String,
    creator_id: u64,
    creator_type: String,
    response: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    asset_uri: Option<String>,
}

impl UploadSummary {
    pub(crate) fn asset_uri(&self) -> Option<&str> {
        self.asset_uri.as_deref()
    }

    pub(crate) fn operation_id(&self) -> Option<&str> {
        self.operation_path.as_deref()
    }

    pub(crate) fn operation_status(&self) -> Option<String> {
        if let Some(operation) = &self.operation {
            if operation_failure(operation).is_some() {
                return Some("failed".into());
            }
            if operation.get("done").and_then(Value::as_bool) == Some(true) {
                return Some("done".into());
            }
        }
        if self.asset_id.is_some() {
            Some("done".into())
        } else if self.operation_path.is_some() {
            Some("pending".into())
        } else {
            None
        }
    }
}

pub fn run(options: UploadOptions) -> AppResult<()> {
    let mut summary = upload_once(&options)?;
    if options.wait || options.import_to.is_some() {
        wait_for_upload(
            &mut summary,
            api_key_from_options(&options)?,
            options.wait_timeout_secs,
        )?;
    }
    if let Some(import_to) = options.import_to {
        let asset_uri = summary.asset_uri.clone().ok_or_else(|| {
            AppError::Other("upload completed but no assetId was returned for import".into())
        })?;
        match options.kind {
            UploadKind::Audio => crate::cli::import_uploaded::run(
                options.port,
                options.studio,
                "audio".into(),
                asset_uri,
                Some(import_to),
                options.name,
                None,
                None,
                None,
                None,
                None,
                false,
                options.json_output,
            )?,
            UploadKind::Image => crate::cli::import_uploaded::run(
                options.port,
                options.studio,
                "image".into(),
                asset_uri,
                Some(import_to),
                options.name,
                Some("image".into()),
                None,
                None,
                None,
                None,
                false,
                options.json_output,
            )?,
            UploadKind::Model | UploadKind::Mesh => {
                return Err(AppError::Other(
                    "--import-to is currently supported for uploaded image and audio assets only"
                        .into(),
                ))
            }
        }
    }

    if options.json_output {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_summary(&summary);
    }
    Ok(())
}

pub(crate) fn upload_image_bytes(options: UploadImageBytesOptions) -> AppResult<UploadSummary> {
    if options.bytes.is_empty() {
        return Err(AppError::Other(
            "cannot upload an empty image asset payload".into(),
        ));
    }

    let profile = resolve_upload_profile_parts(
        options.profile.as_deref(),
        options.creator_id,
        options.api_key.as_deref(),
    )?;
    let creator_id = options
        .creator_id
        .or_else(|| profile.as_ref().map(|profile| profile.creator_id))
        .ok_or_else(|| {
            AppError::Other("missing --creator-id; pass it directly or use --profile".into())
        })?;
    let creator_type = options
        .creator_type
        .clone()
        .or_else(|| profile.as_ref().map(|profile| profile.creator_type.clone()))
        .unwrap_or_else(|| "group".to_string());
    let api_key = options
        .api_key
        .clone()
        .or_else(|| profile.map(|profile| profile.api_key))
        .or_else(|| std::env::var("ROBLOX_API_KEY").ok())
        .ok_or_else(|| {
            AppError::Other(
                "missing Open Cloud API key; pass --api-key, set ROBLOX_API_KEY, or use --profile"
                    .into(),
            )
        })?;
    let upload_name = options.name.clone().unwrap_or_else(|| {
        Path::new(&options.filename)
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("RehostedImage")
            .to_string()
    });

    let filename = options.filename;
    let mime = mime_type("Image", Path::new(&filename));
    let mut summary = upload_bytes_to_open_cloud(
        &api_key,
        "Image",
        creator_id,
        &creator_type,
        filename.clone(),
        filename,
        options.bytes,
        mime,
        upload_name,
        options.description,
    )?;
    if summary.operation_path.is_some() {
        wait_for_upload(&mut summary, api_key, options.wait_timeout_secs)?;
    }
    if summary.asset_uri.is_none() {
        return Err(AppError::Other(
            "Open Cloud image upload completed but did not return an asset id".into(),
        ));
    }
    Ok(summary)
}

fn upload_once(options: &UploadOptions) -> AppResult<UploadSummary> {
    if !options.file.is_file() {
        return Err(AppError::Other(format!(
            "upload file does not exist or is not a file: {}",
            options.file.display()
        )));
    }

    if matches!(options.kind, UploadKind::Mesh) && is_studio_local_geometry_format(&options.file) {
        return Err(AppError::Other(format!(
            "Open Cloud upload does not publish raw {} geometry directly. Use `rs import-asset --file {}` for Studio-local conversion, or upload a Roblox-supported model container such as FBX/glTF/GLB with `rs upload model`.",
            extension(&options.file).unwrap_or("mesh"),
            options.file.display()
        )));
    }

    let profile = resolve_upload_profile(options)?;
    let creator_id = options
        .creator_id
        .or_else(|| profile.as_ref().map(|profile| profile.creator_id))
        .ok_or_else(|| {
            AppError::Other("missing --creator-id; pass it directly or use --profile".into())
        })?;
    let creator_type = options
        .creator_type
        .clone()
        .or_else(|| profile.as_ref().map(|profile| profile.creator_type.clone()))
        .unwrap_or_else(|| "group".to_string());
    let api_key = options
        .api_key
        .clone()
        .or_else(|| profile.map(|profile| profile.api_key))
        .or_else(|| std::env::var("ROBLOX_API_KEY").ok())
        .ok_or_else(|| {
            AppError::Other(
                "missing Open Cloud API key; pass --api-key, set ROBLOX_API_KEY, or use --profile"
                    .into(),
            )
        })?;

    let asset_type = options.kind.asset_type_for_file(&options.file);
    let upload_name = options
        .name
        .clone()
        .unwrap_or_else(|| default_name(&options.file));
    let bytes = std::fs::read(&options.file)?;
    let filename = options
        .file
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("asset")
        .to_string();

    upload_bytes_to_open_cloud(
        &api_key,
        asset_type,
        creator_id,
        &creator_type,
        options.file.display().to_string(),
        filename.clone(),
        bytes,
        mime_type(asset_type, &options.file),
        upload_name,
        options.description.clone(),
    )
}

fn upload_bytes_to_open_cloud(
    api_key: &str,
    asset_type: &'static str,
    creator_id: u64,
    creator_type: &str,
    file_label: String,
    filename: String,
    bytes: Vec<u8>,
    mime_type: &'static str,
    upload_name: String,
    description: Option<String>,
) -> AppResult<UploadSummary> {
    let request = build_request(
        asset_type,
        creator_id,
        creator_type,
        &upload_name,
        description,
    )?;

    let request_part = reqwest::blocking::multipart::Part::text(serde_json::to_string(&request)?)
        .mime_str("application/json")?;
    let file_part = reqwest::blocking::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(mime_type)?;
    let form = reqwest::blocking::multipart::Form::new()
        .part("request", request_part)
        .part("fileContent", file_part);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;
    let response = client
        .post(ASSETS_URL)
        .header("x-api-key", api_key)
        .multipart(form)
        .send()?;
    let status = response.status();
    let body = response.text()?;
    let value = serde_json::from_str::<Value>(&body).unwrap_or_else(|_| json!({ "body": body }));
    if !status.is_success() {
        return Err(AppError::Other(format!(
            "Open Cloud asset upload failed with HTTP {status}: {}",
            redacted_json_string_with_secrets(&value, &[api_key])?
        )));
    }

    let operation_path = value
        .get("path")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let asset_id = extract_asset_id(&value);
    Ok(UploadSummary {
        asset_type,
        file: file_label,
        creator_id,
        creator_type: creator_type.to_string(),
        response: value,
        operation_path,
        operation: None,
        asset_uri: asset_id
            .as_ref()
            .map(|asset_id| format!("rbxassetid://{asset_id}")),
        asset_id,
    })
}

fn wait_for_upload(
    summary: &mut UploadSummary,
    api_key: String,
    timeout_secs: u64,
) -> AppResult<()> {
    let operation_path = summary.operation_path.clone().ok_or_else(|| {
        AppError::Other("upload response did not include an operation path to wait on".into())
    })?;
    let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(1));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    loop {
        let url = format!(
            "{}/{}",
            OPERATIONS_BASE_URL.trim_end_matches('/'),
            operation_path.trim_start_matches('/')
        );
        let response = client.get(&url).header("x-api-key", &api_key).send()?;
        let status = response.status();
        let body = response.text()?;
        let value =
            serde_json::from_str::<Value>(&body).unwrap_or_else(|_| json!({ "body": body }));
        if !status.is_success() {
            return Err(AppError::Other(format!(
                "Open Cloud operation poll failed with HTTP {status}: {}",
                redacted_json_string_with_secrets(&value, &[api_key.as_str()])?
            )));
        }
        if value.get("done").and_then(Value::as_bool) == Some(true) {
            if let Some(status) = operation_failure(&value) {
                return Err(AppError::Other(format!(
                    "Open Cloud upload operation failed: {}",
                    redacted_json_string_with_secrets(&status, &[api_key.as_str()])?
                )));
            }
            apply_completed_operation(summary, value)?;
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(AppError::Other(format!(
                "timed out waiting for Open Cloud operation {operation_path}"
            )));
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn apply_completed_operation(summary: &mut UploadSummary, value: Value) -> AppResult<()> {
    let asset_id = extract_asset_id(&value).or_else(|| extract_asset_id(&summary.response));
    if asset_id.is_none() {
        return Err(AppError::Other(format!(
            "Open Cloud upload operation completed without an asset id: {}",
            redacted_json_string(&value)?
        )));
    }
    summary.asset_uri = asset_id
        .as_ref()
        .map(|asset_id| format!("rbxassetid://{asset_id}"));
    summary.asset_id = asset_id;
    summary.operation = Some(value);
    Ok(())
}

fn operation_failure(value: &Value) -> Option<Value> {
    if let Some(error) = value.get("error") {
        return Some(error.clone());
    }
    if let Some(status) = value.get("status") {
        return Some(status.clone());
    }
    None
}

fn redacted_json_string(value: &Value) -> AppResult<String> {
    redacted_json_string_with_secrets(value, &[])
}

fn redacted_json_string_with_secrets(value: &Value, secrets: &[&str]) -> AppResult<String> {
    let mut text = serde_json::to_string(&redact_json(value))?;
    for secret in secrets {
        if !secret.is_empty() {
            text = text.replace(secret, "<redacted>");
        }
    }
    Ok(text)
}

fn redact_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (key, value) in map {
                if is_sensitive_key(key) {
                    redacted.insert(key.clone(), Value::String("<redacted>".into()));
                } else {
                    redacted.insert(key.clone(), redact_json(value));
                }
            }
            Value::Object(redacted)
        }
        Value::Array(items) => Value::Array(items.iter().map(redact_json).collect()),
        Value::String(text) if looks_sensitive_value(text) => Value::String("<redacted>".into()),
        _ => value.clone(),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("api_key")
        || key.contains("apikey")
        || key.contains("token")
        || key.contains("secret")
        || key.contains("authorization")
        || key.contains("x-api-key")
}

fn looks_sensitive_value(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("roblox_api_key=")
        || lower.contains("x-api-key")
        || lower.contains("authorization:")
        || lower.contains("bearer ")
        || lower.contains("api_key=")
        || lower.contains("apikey=")
        || lower.contains("token=")
}

fn api_key_from_options(options: &UploadOptions) -> AppResult<String> {
    let profile = resolve_upload_profile(options)?;
    options
        .api_key
        .clone()
        .or_else(|| profile.map(|profile| profile.api_key))
        .or_else(|| std::env::var("ROBLOX_API_KEY").ok())
        .ok_or_else(|| AppError::Other("missing Open Cloud API key for operation polling".into()))
}

fn resolve_upload_profile(
    options: &UploadOptions,
) -> AppResult<Option<crate::cli::auth::OpenCloudProfile>> {
    resolve_upload_profile_parts(
        options.profile.as_deref(),
        options.creator_id,
        options.api_key.as_deref(),
    )
}

fn resolve_upload_profile_parts(
    profile: Option<&str>,
    creator_id: Option<u64>,
    api_key: Option<&str>,
) -> AppResult<Option<crate::cli::auth::OpenCloudProfile>> {
    if profile.is_some() {
        return crate::cli::auth::resolve_profile(profile);
    }
    if creator_id.is_none() || api_key.is_none() {
        if let Some(profile) = crate::cli::auth::resolve_profile(Some("default"))
            .ok()
            .flatten()
        {
            return Ok(Some(profile));
        }
    }
    Ok(None)
}

impl UploadKind {
    fn asset_type_for_file(self, file: &Path) -> &'static str {
        match self {
            UploadKind::Image => "Image",
            UploadKind::Audio => "Audio",
            UploadKind::Model => "Model",
            UploadKind::Mesh => match extension(file).map(|value| value.to_ascii_lowercase()) {
                Some(ext) if ext == "mesh" || ext == "meshdata" => "Mesh",
                _ => "Model",
            },
        }
    }
}

fn build_request(
    asset_type: &'static str,
    creator_id: u64,
    creator_type: &str,
    name: &str,
    description: Option<String>,
) -> AppResult<Value> {
    let creator = match creator_type {
        "group" => json!({ "groupId": creator_id.to_string() }),
        "user" => json!({ "userId": creator_id.to_string() }),
        other => {
            return Err(AppError::Other(format!(
                "unsupported creator type '{other}'; expected group or user"
            )))
        }
    };
    Ok(json!({
        "assetType": asset_type,
        "displayName": name,
        "description": description.unwrap_or_default(),
        "creationContext": {
            "creator": creator
        }
    }))
}

fn default_name(file: &Path) -> String {
    file.file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("UploadedAsset")
        .to_string()
}

fn extension(file: &Path) -> Option<&str> {
    file.extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn is_studio_local_geometry_format(file: &Path) -> bool {
    matches!(
        extension(file).map(|value| value.to_ascii_lowercase()),
        Some(ext) if matches!(ext.as_str(), "obj" | "stl")
    )
}

fn mime_type(asset_type: &str, file: &Path) -> &'static str {
    match asset_type {
        "Image" => match extension(file).map(|value| value.to_ascii_lowercase()) {
            Some(ext) if ext == "jpg" || ext == "jpeg" => "image/jpeg",
            Some(ext) if ext == "bmp" => "image/bmp",
            Some(ext) if ext == "tga" => "image/tga",
            _ => "image/png",
        },
        "Audio" => match extension(file).map(|value| value.to_ascii_lowercase()) {
            Some(ext) if ext == "mp3" => "audio/mpeg",
            Some(ext) if ext == "ogg" => "audio/ogg",
            Some(ext) if ext == "flac" => "audio/flac",
            _ => "audio/wav",
        },
        "Mesh" => "model/x-file-mesh-data",
        "Model" => match extension(file).map(|value| value.to_ascii_lowercase()) {
            Some(ext) if ext == "fbx" => "model/fbx",
            Some(ext) if ext == "gltf" => "model/gltf+json",
            Some(ext) if ext == "glb" => "model/gltf-binary",
            Some(ext) if ext == "rbxm" || ext == "rbxmx" => "model/x-rbxm",
            _ => "application/octet-stream",
        },
        _ => "application/octet-stream",
    }
}

fn extract_asset_id(value: &Value) -> Option<String> {
    value
        .get("assetId")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("response")
                .and_then(|response| response.get("assetId"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            value
                .get("response")
                .and_then(|response| response.get("path"))
                .and_then(Value::as_str)
                .and_then(|path| path.rsplit('/').next())
        })
        .map(ToOwned::to_owned)
}

fn print_summary(summary: &UploadSummary) {
    println!("Uploaded {}: {}", summary.asset_type, summary.file);
    println!("Creator: {} {}", summary.creator_type, summary.creator_id);
    if let Some(asset_uri) = &summary.asset_uri {
        println!("Asset: {asset_uri}");
    } else if let Some(path) = &summary.operation_path {
        println!("Operation: {path}");
    } else {
        println!(
            "Response: {}",
            serde_json::to_string(&summary.response).unwrap_or_else(|_| "<unprintable>".into())
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_completed_operation, build_request, extract_asset_id, mime_type, operation_failure,
        redacted_json_string, redacted_json_string_with_secrets, UploadKind, UploadSummary,
    };
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn upload_kind_maps_mesh_to_model_asset_type() {
        assert_eq!(
            UploadKind::Image.asset_type_for_file(Path::new("icon.png")),
            "Image"
        );
        assert_eq!(
            UploadKind::Audio.asset_type_for_file(Path::new("click.wav")),
            "Audio"
        );
        assert_eq!(
            UploadKind::Mesh.asset_type_for_file(Path::new("model.glb")),
            "Model"
        );
        assert_eq!(
            UploadKind::Mesh.asset_type_for_file(Path::new("data.mesh")),
            "Mesh"
        );
    }

    #[test]
    fn build_request_uses_creator_shape() {
        let value = build_request("Image", 123, "group", "Icon", Some("desc".into())).unwrap();
        assert_eq!(value["assetType"], "Image");
        assert_eq!(value["displayName"], "Icon");
        assert_eq!(value["creationContext"]["creator"]["groupId"], "123");
    }

    #[test]
    fn model_mime_types_follow_open_cloud_formats() {
        assert_eq!(mime_type("Model", Path::new("a.fbx")), "model/fbx");
        assert_eq!(mime_type("Model", Path::new("a.gltf")), "model/gltf+json");
        assert_eq!(mime_type("Model", Path::new("a.glb")), "model/gltf-binary");
    }

    #[test]
    fn extracts_asset_id_from_done_operation() {
        let value = json!({
            "done": true,
            "response": { "assetId": "2205400862" }
        });
        assert_eq!(extract_asset_id(&value).as_deref(), Some("2205400862"));
    }

    #[test]
    fn completed_operation_requires_asset_id() {
        let mut summary = UploadSummary {
            asset_type: "Image",
            file: "icon.png".into(),
            creator_id: 1,
            creator_type: "group".into(),
            response: json!({ "path": "operations/upload" }),
            operation_path: Some("operations/upload".into()),
            operation: None,
            asset_id: None,
            asset_uri: None,
        };
        let err = apply_completed_operation(&mut summary, json!({ "done": true })).unwrap_err();
        assert!(err.to_string().contains("without an asset id"));
    }

    #[test]
    fn parses_operation_failure_bodies() {
        let failure = json!({
            "done": true,
            "error": {
                "code": "PERMISSION_DENIED",
                "message": "creator cannot upload this asset"
            }
        });
        assert_eq!(
            operation_failure(&failure).unwrap()["code"],
            "PERMISSION_DENIED"
        );
    }

    #[test]
    fn redacts_secretish_upload_errors() {
        let body = json!({
            "apiKey": "secret-profile-key",
            "message": "ROBLOX_API_KEY=secret-profile-key failed",
            "safe": "permission denied"
        });
        let text = redacted_json_string(&body).unwrap();
        assert!(!text.contains("secret-profile-key"));
        assert!(text.contains("<redacted>"));
        assert!(text.contains("permission denied"));
    }

    #[test]
    fn redacts_known_api_key_echoes_without_secretish_field_names() {
        let body = json!({
            "message": "request used plain-secret-value"
        });
        let text = redacted_json_string_with_secrets(&body, &["plain-secret-value"]).unwrap();
        assert!(!text.contains("plain-secret-value"));
        assert!(text.contains("<redacted>"));
    }
}
