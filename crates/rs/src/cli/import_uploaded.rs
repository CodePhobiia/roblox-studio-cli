use crate::error::{AppError, AppResult};
use crate::protocol::messages::{ImportUploadedRequest, ImportUploadedResponse};

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
            source_id: None,
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
    if value.starts_with("rbxassetid://") {
        return Ok(value.to_string());
    }
    if value.chars().all(|ch| ch.is_ascii_digit()) {
        return Ok(format!("rbxassetid://{value}"));
    }
    Err(AppError::Other(format!(
        "asset id must be numeric or rbxassetid://..., got '{value}'"
    )))
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
    use super::normalize_asset_id;

    #[test]
    fn normalizes_numeric_asset_id() {
        assert_eq!(normalize_asset_id("123").unwrap(), "rbxassetid://123");
        assert_eq!(
            normalize_asset_id("rbxassetid://456").unwrap(),
            "rbxassetid://456"
        );
    }
}
