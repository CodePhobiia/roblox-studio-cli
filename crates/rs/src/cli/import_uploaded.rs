use crate::error::{AppError, AppResult};
use crate::protocol::messages::{ImportUploadedRequest, ImportUploadedResponse};
use std::path::{Path, PathBuf};

#[allow(clippy::too_many_arguments)]
pub fn run(
    port: u16,
    studio: Option<String>,
    kind: String,
    asset_id: String,
    to: Option<String>,
    name: Option<String>,
    ui_kind: Option<String>,
    size: Option<String>,
    position: Option<String>,
    volume: Option<f32>,
    playback_speed: Option<f32>,
    looped: bool,
    json: bool,
) -> AppResult<()> {
    let normalized_asset_id = normalize_asset_id(&asset_id)?;
    if kind == "audio" {
        validate_sound_options(volume, playback_speed)?;
    }
    let default_parent = if kind == "audio" {
        "SoundService"
    } else {
        "StarterGui"
    };
    let parent_path = to.unwrap_or_else(|| default_parent.to_string());
    let name = name.unwrap_or_else(|| default_name(&normalized_asset_id, &kind));
    let (ui_width, ui_height) = match size {
        Some(size) => {
            let (w, h) = crate::cli::import_image::parse_size(&size)?;
            (Some(w), Some(h))
        }
        None if kind == "image" => (Some(128), Some(128)),
        None => (None, None),
    };
    let (position_x, position_y) = match position {
        Some(position) => {
            let (x, y) = crate::cli::import_image::parse_position(&position)?;
            (Some(x), Some(y))
        }
        None if kind == "image" => (Some(0), Some(0)),
        None => (None, None),
    };
    let source_id = stable_source_id(&format!("uploaded-{kind}"), &[normalized_asset_id.clone()]);

    let response: ImportUploadedResponse = crate::cli::request::post(
        port,
        "import-uploaded",
        "/import-uploaded",
        &ImportUploadedRequest {
            studio,
            parent_path,
            kind,
            name,
            asset_id: normalized_asset_id,
            ui_kind,
            ui_width,
            ui_height,
            position_x,
            position_y,
            volume,
            playback_speed,
            looped,
            source_id: Some(source_id),
        },
        75,
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "Imported uploaded {} at {}",
            response.class_name, response.instance_path
        );
        if !response.warnings.is_empty() {
            println!("Warnings ({}):", response.warnings.len());
            for warning in response.warnings {
                println!("  - {warning}");
            }
        }
    }
    Ok(())
}

pub(crate) fn normalize_asset_id(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::Other("asset id must not be empty".into()));
    }
    if let Some(id) = value.strip_prefix("rbxassetid://") {
        if id.chars().all(|ch| ch.is_ascii_digit()) && !id.is_empty() {
            return Ok(value.to_string());
        }
        return Err(AppError::Other(format!(
            "asset id URI must look like rbxassetid://123, got '{value}'"
        )));
    }
    if let Some(id) = extract_id_query(value) {
        return Ok(format!("rbxassetid://{id}"));
    }
    if value.chars().all(|ch| ch.is_ascii_digit()) {
        return Ok(format!("rbxassetid://{value}"));
    }
    Err(AppError::Other(format!(
        "asset id must be numeric or rbxassetid://..., got '{value}'"
    )))
}

pub(crate) fn validate_sound_options(
    volume: Option<f32>,
    playback_speed: Option<f32>,
) -> AppResult<()> {
    if let Some(volume) = volume {
        if !volume.is_finite() || !(0.0..=10.0).contains(&volume) {
            return Err(AppError::Other(
                "Sound.Volume must be a finite number between 0 and 10".into(),
            ));
        }
    }
    if let Some(playback_speed) = playback_speed {
        if !playback_speed.is_finite() || playback_speed <= 0.0 {
            return Err(AppError::Other(
                "Sound.PlaybackSpeed must be a positive finite number".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn stable_file_source_id(prefix: &str, path: &Path) -> AppResult<String> {
    let identity = stable_path_identity(path);
    let hash = file_content_hash(path)?;
    Ok(stable_source_id(prefix, &[identity, hash]))
}

pub(crate) fn file_content_hash(path: &Path) -> AppResult<String> {
    Ok(format!("{:016x}", fnv1a64(&std::fs::read(path)?)))
}

pub(crate) fn stable_source_id(prefix: &str, parts: &[String]) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(prefix.as_bytes());
    for part in parts {
        bytes.push(0);
        bytes.extend_from_slice(part.as_bytes());
    }
    format!("rs:{prefix}:{:016x}", fnv1a64(&bytes))
}

fn stable_path_identity(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| PathBuf::from(path))
        .to_string_lossy()
        .replace('\\', "/")
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn extract_id_query(value: &str) -> Option<String> {
    let marker = "id=";
    let start = value.find(marker)? + marker.len();
    let digits = value[start..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    (!digits.is_empty()).then_some(digits)
}

fn default_name(asset_id: &str, kind: &str) -> String {
    let suffix = asset_id
        .strip_prefix("rbxassetid://")
        .unwrap_or(asset_id)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    let prefix = if kind == "audio" { "Sound" } else { "Image" };
    if suffix.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}_{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_asset_id, stable_source_id, validate_sound_options};

    #[test]
    fn normalizes_numeric_asset_id() {
        assert_eq!(normalize_asset_id("123").unwrap(), "rbxassetid://123");
        assert_eq!(
            normalize_asset_id("rbxassetid://456").unwrap(),
            "rbxassetid://456"
        );
        assert_eq!(
            normalize_asset_id("https://www.roblox.com/asset/?id=789").unwrap(),
            "rbxassetid://789"
        );
    }

    #[test]
    fn rejects_invalid_asset_id_uri() {
        assert!(normalize_asset_id("rbxassetid://abc").is_err());
        assert!(normalize_asset_id("https://www.roblox.com/asset/?id=abc").is_err());
        assert!(normalize_asset_id("rbxasset://sounds/click.wav").is_err());
    }

    #[test]
    fn validates_sound_option_ranges() {
        validate_sound_options(Some(0.0), Some(0.1)).unwrap();
        validate_sound_options(Some(10.0), Some(3.0)).unwrap();
        assert!(validate_sound_options(Some(-0.1), None).is_err());
        assert!(validate_sound_options(Some(10.1), None).is_err());
        assert!(validate_sound_options(None, Some(0.0)).is_err());
        assert!(validate_sound_options(None, Some(f32::NAN)).is_err());
    }

    #[test]
    fn stable_source_ids_are_content_addressed_by_parts() {
        let first = stable_source_id("uploaded-audio", &["rbxassetid://123".to_string()]);
        let second = stable_source_id("uploaded-audio", &["rbxassetid://123".to_string()]);
        let different = stable_source_id("uploaded-audio", &["rbxassetid://456".to_string()]);
        assert_eq!(first, second);
        assert_ne!(first, different);
    }
}
