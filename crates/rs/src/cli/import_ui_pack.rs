use crate::error::{AppError, AppResult};
use crate::protocol::messages::{ImportUiPackElement, ImportUiPackRequest, ImportUiPackResponse};
use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiPackManifest {
    name: Option<String>,
    to: Option<String>,
    elements: Vec<UiPackManifestElement>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiPackManifestElement {
    file: PathBuf,
    name: Option<String>,
    kind: Option<String>,
    size: Option<String>,
    position: Option<String>,
    anchor: Option<String>,
    z_index: Option<i32>,
    scale_type: Option<String>,
    background_transparency: Option<f32>,
}

pub fn run(
    port: u16,
    studio: Option<String>,
    folder: Option<PathBuf>,
    manifest: Option<PathBuf>,
    to: Option<String>,
    name: Option<String>,
    kind: String,
    json: bool,
) -> AppResult<()> {
    let (root, request_name, parent_path, element_specs) =
        load_pack_inputs(folder, manifest, to, name, kind)?;
    let mut elements = Vec::with_capacity(element_specs.len());
    let mut warnings = Vec::<String>::new();
    let mut source_parts = vec![
        root.canonicalize()
            .unwrap_or_else(|_| root.clone())
            .to_string_lossy()
            .replace('\\', "/"),
        request_name.clone(),
    ];
    for spec in element_specs {
        let path = root.join(&spec.file);
        source_parts.push(format!(
            "{}:{}",
            spec.file.to_string_lossy().replace('\\', "/"),
            crate::cli::import_uploaded::file_content_hash(&path)?
        ));
        let image = crate::cli::import_image::load_png(&path)?;
        warnings.extend(image.warnings.clone());
        let element_name = spec
            .name
            .unwrap_or_else(|| crate::cli::import_image::file_stem(&path));
        let (size_scale_x, size_offset_x, size_scale_y, size_offset_y) = match spec.size {
            Some(value) => parse_udim2_size(&value)?,
            None if spec.kind.as_deref() == Some("icon") => (0.0, 64, 0.0, 64),
            None => (0.0, image.width as i32, 0.0, image.height as i32),
        };
        let (position_scale_x, position_offset_x, position_scale_y, position_offset_y) =
            parse_udim2_position(spec.position.as_deref().unwrap_or("0,0"))?;
        let (anchor_x, anchor_y) =
            parse_pair_f32(spec.anchor.as_deref().unwrap_or("0,0"), "anchor")?;
        elements.push(ImportUiPackElement {
            name: element_name,
            kind: spec.kind.unwrap_or_else(|| "image".to_string()),
            width: image.width,
            height: image.height,
            size_scale_x,
            size_offset_x,
            size_scale_y,
            size_offset_y,
            position_scale_x,
            position_offset_x,
            position_scale_y,
            position_offset_y,
            anchor_x,
            anchor_y,
            z_index: spec.z_index,
            scale_type: spec.scale_type,
            background_transparency: spec.background_transparency,
            pixels_base64: crate::cli::import_image::encode_base64(&image.rgba),
        });
    }

    let mut response: ImportUiPackResponse = crate::cli::request::post(
        port,
        "import-ui-pack",
        "/import-ui-pack",
        &ImportUiPackRequest {
            studio,
            parent_path,
            name: request_name,
            elements,
            source_id: Some(crate::cli::import_uploaded::stable_source_id(
                "ui-pack",
                &source_parts,
            )),
        },
        210,
    )?;
    response.warnings.splice(0..0, warnings);

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "Imported {} UI element(s) into {}",
            response.element_count, response.gui_path
        );
        for path in response.element_paths.iter().take(20) {
            println!("  - {path}");
        }
        if !response.warnings.is_empty() {
            println!("Warnings ({}):", response.warnings.len());
            for warning in response.warnings.iter().take(20) {
                println!("  - {warning}");
            }
        }
    }
    std::io::stdout().flush()?;
    Ok(())
}

fn load_pack_inputs(
    folder: Option<PathBuf>,
    manifest: Option<PathBuf>,
    to: Option<String>,
    name: Option<String>,
    kind: String,
) -> AppResult<(PathBuf, String, String, Vec<UiPackManifestElement>)> {
    if let Some(manifest_path) = manifest {
        let manifest: UiPackManifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
        let root = manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let request_name = name.or(manifest.name).unwrap_or_else(|| {
            root.file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("ImportedGui")
                .to_string()
        });
        let parent_path = to
            .or(manifest.to)
            .unwrap_or_else(|| "StarterGui".to_string());
        return Ok((root, request_name, parent_path, manifest.elements));
    }

    let folder =
        folder.ok_or_else(|| AppError::Other("--folder or --manifest is required".into()))?;
    let request_name = name.unwrap_or_else(|| {
        folder
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("ImportedGui")
            .to_string()
    });
    let parent_path = to.unwrap_or_else(|| "StarterGui".to_string());
    let mut files = fs::read_dir(&folder)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|v| v.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        })
        .collect::<Vec<_>>();
    files.sort();
    if files.is_empty() {
        return Err(AppError::Other(format!(
            "no PNG files found in {}",
            folder.display()
        )));
    }
    let elements = files
        .into_iter()
        .map(|file| UiPackManifestElement {
            file: file
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| file.clone()),
            name: file
                .file_stem()
                .and_then(|v| v.to_str())
                .map(ToOwned::to_owned),
            kind: Some(kind.clone()),
            size: None,
            position: None,
            anchor: None,
            z_index: None,
            scale_type: None,
            background_transparency: None,
        })
        .collect();
    Ok((folder, request_name, parent_path, elements))
}

fn parse_udim2_size(value: &str) -> AppResult<(f32, i32, f32, i32)> {
    if value.contains('x') || value.contains('X') {
        let (w, h) = crate::cli::import_image::parse_size(value)?;
        return Ok((0.0, w as i32, 0.0, h as i32));
    }
    let (x, y) = parse_pair_f32(value, "size")?;
    Ok((x, 0, y, 0))
}

fn parse_udim2_position(value: &str) -> AppResult<(f32, i32, f32, i32)> {
    let parts = value.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(AppError::Other("position must look like 0,0".into()));
    }
    let x = parts[0]
        .parse::<f32>()
        .map_err(|_| AppError::Other("position x must be a number".into()))?;
    let y = parts[1]
        .parse::<f32>()
        .map_err(|_| AppError::Other("position y must be a number".into()))?;
    let scale_like =
        parts.iter().any(|part| part.contains('.')) && x.abs() <= 1.0 && y.abs() <= 1.0;
    if scale_like {
        Ok((x, 0, y, 0))
    } else {
        Ok((0.0, x.round() as i32, 0.0, y.round() as i32))
    }
}

fn parse_pair_f32(value: &str, label: &str) -> AppResult<(f32, f32)> {
    let (x, y) = value
        .split_once(',')
        .ok_or_else(|| AppError::Other(format!("{label} must look like 0,0")))?;
    let x = x
        .trim()
        .parse::<f32>()
        .map_err(|_| AppError::Other(format!("{label} x must be a number")))?;
    let y = y
        .trim()
        .parse::<f32>()
        .map_err(|_| AppError::Other(format!("{label} y must be a number")))?;
    Ok((x, y))
}

#[cfg(test)]
mod tests {
    use super::{parse_udim2_position, parse_udim2_size};

    #[test]
    fn parses_pixel_size() {
        assert_eq!(parse_udim2_size("100x50").unwrap(), (0.0, 100, 0.0, 50));
    }

    #[test]
    fn parses_scale_position() {
        assert_eq!(parse_udim2_position("0.5,0.25").unwrap(), (0.5, 0, 0.25, 0));
    }
}
