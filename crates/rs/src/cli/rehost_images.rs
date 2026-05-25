use crate::error::{AppError, AppResult};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

const ASSET_DELIVERY_BASE_URL: &str = "https://apis.roblox.com/asset-delivery-api/v1/assetId";
const MAX_DELIVERY_HOPS: usize = 3;

#[derive(Debug, Clone)]
pub struct ImageRehostOptions {
    pub creator_id: Option<u64>,
    pub creator_type: Option<String>,
    pub profile: Option<String>,
    pub api_key: Option<String>,
    pub source_api_key: Option<String>,
    pub wait_timeout_secs: u64,
    pub dry_run: bool,
    pub quiet: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRehostReport {
    pub scanned_image_properties: usize,
    pub unique_asset_count: usize,
    pub rehosted_asset_count: usize,
    pub rewritten_property_count: usize,
    pub dry_run: bool,
    pub planned_asset_ids: Vec<String>,
    pub mappings: Vec<ImageRehostMapping>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRehostMapping {
    pub source_asset_id: String,
    pub source_uri: String,
    pub target_uri: String,
    pub property_count: usize,
}

#[derive(Debug, Clone)]
struct ImageRefSummary {
    source_uri: String,
    property_count: usize,
}

#[derive(Debug, Clone)]
struct RehostAuth {
    creator_id: u64,
    creator_type: String,
    upload_api_key: String,
    source_api_key: String,
}

#[derive(Debug)]
struct DownloadedImage {
    bytes: Vec<u8>,
    extension: &'static str,
    resolved_asset_id: String,
}

pub fn rehost_image_refs_in_blob(
    blob: &mut Value,
    options: &ImageRehostOptions,
) -> AppResult<ImageRehostReport> {
    let refs = collect_image_refs(blob);
    let planned_asset_ids = refs.keys().cloned().collect::<Vec<_>>();
    let scanned_image_properties = refs.values().map(|item| item.property_count).sum();
    if refs.is_empty() || options.dry_run {
        return Ok(ImageRehostReport {
            scanned_image_properties,
            unique_asset_count: refs.len(),
            rehosted_asset_count: 0,
            rewritten_property_count: 0,
            dry_run: options.dry_run,
            planned_asset_ids,
            mappings: Vec::new(),
        });
    }

    let auth = resolve_rehost_auth(options)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;
    let mut uri_mapping = BTreeMap::new();
    let mut mappings = Vec::new();

    if !options.quiet {
        println!("Rehosting {} image asset(s)...", refs.len());
    }

    for (source_asset_id, summary) in &refs {
        let downloaded = download_image_asset(&client, source_asset_id, &auth.source_api_key)?;
        let target_name = format!("rs-rehost-{source_asset_id}");
        let filename = format!(
            "rs-rehost-{}.{}",
            downloaded.resolved_asset_id, downloaded.extension
        );
        let upload =
            crate::cli::upload::upload_image_bytes(crate::cli::upload::UploadImageBytesOptions {
                bytes: downloaded.bytes,
                filename,
                creator_id: Some(auth.creator_id),
                creator_type: Some(auth.creator_type.clone()),
                profile: None,
                name: Some(target_name),
                description: Some(format!("Rehosted by rs from asset {source_asset_id}")),
                api_key: Some(auth.upload_api_key.clone()),
                wait_timeout_secs: options.wait_timeout_secs,
            })?;
        let target_uri = upload.asset_uri().ok_or_else(|| {
            AppError::Other(format!(
                "rehosted image {source_asset_id} but upload returned no target asset URI"
            ))
        })?;
        uri_mapping.insert(source_asset_id.clone(), target_uri.to_string());
        mappings.push(ImageRehostMapping {
            source_asset_id: source_asset_id.clone(),
            source_uri: summary.source_uri.clone(),
            target_uri: target_uri.to_string(),
            property_count: summary.property_count,
        });
        if !options.quiet {
            println!("  - {} -> {}", summary.source_uri, target_uri);
        }
    }

    let rewritten_property_count = apply_rehost_mapping(blob, &uri_mapping);
    Ok(ImageRehostReport {
        scanned_image_properties,
        unique_asset_count: refs.len(),
        rehosted_asset_count: mappings.len(),
        rewritten_property_count,
        dry_run: false,
        planned_asset_ids,
        mappings,
    })
}

pub fn print_rehost_summary(report: &ImageRehostReport) {
    if report.unique_asset_count == 0 {
        println!("Image rehost: no image asset references found.");
    } else if report.dry_run {
        println!(
            "Image rehost: {} image asset(s) would be rehosted; skipped during dry-run.",
            report.unique_asset_count
        );
    } else {
        println!(
            "Image rehost: {} asset(s), {} property reference(s) rewritten.",
            report.rehosted_asset_count, report.rewritten_property_count
        );
    }
}

fn resolve_rehost_auth(options: &ImageRehostOptions) -> AppResult<RehostAuth> {
    let profile = if options.profile.is_some() {
        crate::cli::auth::resolve_profile(options.profile.as_deref())?
    } else if options.creator_id.is_none() || options.api_key.is_none() {
        crate::cli::auth::resolve_profile(Some("default"))
            .ok()
            .flatten()
    } else {
        None
    };
    let creator_id = options
        .creator_id
        .or_else(|| profile.as_ref().map(|profile| profile.creator_id))
        .ok_or_else(|| {
            AppError::Other(
                "missing target creator for --rehost-images; pass --creator-id or --profile".into(),
            )
        })?;
    let creator_type = options
        .creator_type
        .clone()
        .or_else(|| profile.as_ref().map(|profile| profile.creator_type.clone()))
        .unwrap_or_else(|| "group".to_string());
    let upload_api_key = options
        .api_key
        .clone()
        .or_else(|| profile.map(|profile| profile.api_key))
        .or_else(|| std::env::var("ROBLOX_API_KEY").ok())
        .ok_or_else(|| {
            AppError::Other(
                "missing Open Cloud API key for --rehost-images; pass --api-key, set ROBLOX_API_KEY, or use --profile".into(),
            )
        })?;
    let source_api_key = options
        .source_api_key
        .clone()
        .unwrap_or_else(|| upload_api_key.clone());
    Ok(RehostAuth {
        creator_id,
        creator_type,
        upload_api_key,
        source_api_key,
    })
}

fn collect_image_refs(blob: &Value) -> BTreeMap<String, ImageRefSummary> {
    let mut refs = BTreeMap::<String, ImageRefSummary>::new();
    let Some(instances) = blob.get("instances").and_then(Value::as_object) else {
        return refs;
    };

    for spec in instances.values() {
        let class_name = spec
            .get("className")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(properties) = spec.get("properties").and_then(Value::as_object) else {
            continue;
        };
        for (property, value) in properties {
            if !is_image_asset_property(class_name, property) {
                continue;
            }
            let Some(uri) = value.as_str() else {
                continue;
            };
            let Some(asset_id) = extract_asset_id(uri) else {
                continue;
            };
            refs.entry(asset_id)
                .and_modify(|entry| entry.property_count += 1)
                .or_insert_with(|| ImageRefSummary {
                    source_uri: uri.to_string(),
                    property_count: 1,
                });
        }
    }
    refs
}

fn apply_rehost_mapping(blob: &mut Value, uri_mapping: &BTreeMap<String, String>) -> usize {
    let mut rewritten = 0usize;
    let Some(instances) = blob.get_mut("instances").and_then(Value::as_object_mut) else {
        return rewritten;
    };

    for spec in instances.values_mut() {
        let class_name = spec
            .get("className")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let Some(properties) = spec.get_mut("properties").and_then(Value::as_object_mut) else {
            continue;
        };
        for (property, value) in properties {
            if !is_image_asset_property(&class_name, property) {
                continue;
            }
            let Some(uri) = value.as_str() else {
                continue;
            };
            let Some(source_asset_id) = extract_asset_id(uri) else {
                continue;
            };
            if let Some(target_uri) = uri_mapping.get(&source_asset_id) {
                *value = Value::String(target_uri.clone());
                rewritten += 1;
            }
        }
    }
    rewritten
}

fn is_image_asset_property(class_name: &str, property: &str) -> bool {
    match property {
        "Image" => matches!(class_name, "ImageLabel" | "ImageButton"),
        "Texture" => matches!(
            class_name,
            "Decal" | "Texture" | "ParticleEmitter" | "Beam" | "Trail"
        ),
        "TextureID" | "TextureId" => matches!(class_name, "MeshPart" | "SpecialMesh"),
        "BottomImage" | "MidImage" | "TopImage" => class_name == "ScrollingFrame",
        _ => false,
    }
}

fn download_image_asset(
    client: &reqwest::blocking::Client,
    asset_id: &str,
    api_key: &str,
) -> AppResult<DownloadedImage> {
    let mut current = asset_id.to_string();
    let mut visited = BTreeSet::new();
    for _ in 0..MAX_DELIVERY_HOPS {
        if !visited.insert(current.clone()) {
            return Err(AppError::Other(format!(
                "asset delivery returned a reference cycle while resolving image asset {asset_id}"
            )));
        }
        let url = format!("{ASSET_DELIVERY_BASE_URL}/{current}");
        let response = client.get(&url).header("x-api-key", api_key).send()?;
        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let bytes = response.bytes()?.to_vec();
        if !status.is_success() {
            return Err(AppError::Other(format!(
                "Open Cloud asset delivery failed for {current} with HTTP {status}: {}",
                body_excerpt(&bytes)
            )));
        }
        if is_image_payload(&content_type, &bytes) {
            return Ok(DownloadedImage {
                extension: infer_image_extension(&content_type, &bytes),
                bytes,
                resolved_asset_id: current,
            });
        }
        if let Some(next_asset_id) = find_nested_asset_id(&bytes) {
            current = next_asset_id;
            continue;
        }
        return Err(AppError::Other(format!(
            "asset {current} did not resolve to image bytes; content type was '{}'",
            if content_type.is_empty() {
                "<missing>"
            } else {
                &content_type
            }
        )));
    }
    Err(AppError::Other(format!(
        "asset delivery exceeded {MAX_DELIVERY_HOPS} hops while resolving image asset {asset_id}"
    )))
}

fn is_image_payload(content_type: &str, bytes: &[u8]) -> bool {
    content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().starts_with("image/"))
        || bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(b"\xff\xd8\xff")
        || bytes.starts_with(b"BM")
}

fn infer_image_extension(content_type: &str, bytes: &[u8]) -> &'static str {
    let content_type = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match content_type.as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/bmp" => "bmp",
        "image/tga" | "image/x-tga" => "tga",
        "image/png" => "png",
        _ if bytes.starts_with(b"\xff\xd8\xff") => "jpg",
        _ if bytes.starts_with(b"BM") => "bmp",
        _ => "png",
    }
}

fn extract_asset_id(value: &str) -> Option<String> {
    let value = value.trim();
    if value.chars().all(|ch| ch.is_ascii_digit()) && !value.is_empty() {
        return Some(value.to_string());
    }
    for marker in ["rbxassetid://", "id=", "/assetId/"] {
        if let Some(asset_id) = find_id_after_marker(value, marker) {
            return Some(asset_id);
        }
    }
    None
}

fn find_nested_asset_id(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    for marker in ["rbxassetid://", "id=", "/assetId/"] {
        if let Some(asset_id) = find_id_after_marker(text, marker) {
            return Some(asset_id);
        }
    }
    None
}

fn find_id_after_marker(value: &str, marker: &str) -> Option<String> {
    let mut offset = 0usize;
    while let Some(index) = value[offset..].find(marker) {
        let start = offset + index + marker.len();
        if let Some(asset_id) = take_digits(&value[start..]) {
            return Some(asset_id);
        }
        offset = start;
    }
    None
}

fn take_digits(value: &str) -> Option<String> {
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

fn body_excerpt(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .take(300)
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        apply_rehost_mapping, collect_image_refs, extract_asset_id, find_nested_asset_id,
        infer_image_extension,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn collects_and_rewrites_transfer_blob_image_refs() {
        let mut blob = json!({
            "version": 1,
            "root": "i0",
            "instances": {
                "i0": {
                    "className": "ScreenGui",
                    "properties": { "Name": "Shop" }
                },
                "i1": {
                    "className": "ImageLabel",
                    "properties": { "Name": "Icon", "Image": "rbxassetid://111" }
                },
                "i2": {
                    "className": "ScrollingFrame",
                    "properties": {
                        "BottomImage": "https://www.roblox.com/asset/?id=222",
                        "ScrollBarImageColor3": ["Color3", 1.0, 1.0, 1.0]
                    }
                },
                "i3": {
                    "className": "TextLabel",
                    "properties": { "Text": "rbxassetid://333" }
                }
            }
        });
        let refs = collect_image_refs(&blob);
        assert_eq!(refs.len(), 2);
        assert!(refs.contains_key("111"));
        assert!(refs.contains_key("222"));

        let rewritten = apply_rehost_mapping(
            &mut blob,
            &BTreeMap::from([
                ("111".to_string(), "rbxassetid://9001".to_string()),
                ("222".to_string(), "rbxassetid://9002".to_string()),
            ]),
        );
        assert_eq!(rewritten, 2);
        assert_eq!(
            blob["instances"]["i1"]["properties"]["Image"],
            "rbxassetid://9001"
        );
        assert_eq!(
            blob["instances"]["i2"]["properties"]["BottomImage"],
            "rbxassetid://9002"
        );
        assert_eq!(
            blob["instances"]["i3"]["properties"]["Text"],
            "rbxassetid://333"
        );
    }

    #[test]
    fn extracts_nested_asset_id_from_decal_xml() {
        let xml = br#"<roblox><url>http://www.roblox.com/asset/?id=456</url></roblox>"#;
        assert_eq!(find_nested_asset_id(xml).as_deref(), Some("456"));
    }

    #[test]
    fn extracts_asset_ids_from_common_uri_shapes() {
        assert_eq!(extract_asset_id("123").as_deref(), Some("123"));
        assert_eq!(
            extract_asset_id("rbxassetid://456?width=420").as_deref(),
            Some("456")
        );
        assert_eq!(
            extract_asset_id("https://www.roblox.com/asset/?id=789").as_deref(),
            Some("789")
        );
        assert_eq!(extract_asset_id("rbxasset://textures/ui.png"), None);
    }

    #[test]
    fn infers_image_extension_from_content_type_or_signature() {
        assert_eq!(infer_image_extension("image/jpeg", b""), "jpg");
        assert_eq!(infer_image_extension("", b"\x89PNG\r\n\x1a\nbytes"), "png");
        assert_eq!(infer_image_extension("", b"\xff\xd8\xffrest"), "jpg");
    }
}
