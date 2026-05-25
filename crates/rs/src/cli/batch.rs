use crate::error::{AppError, AppResult};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct BatchManifest {
    steps: Vec<Value>,
}

pub fn run(port: u16, file: PathBuf, dry_run: bool, continue_on_error: bool) -> AppResult<()> {
    let root = file
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let manifest: BatchManifest = serde_json::from_str(&std::fs::read_to_string(&file)?)?;
    let mut ok_count = 0usize;
    let mut failed_count = 0usize;
    for (index, step) in manifest.steps.iter().enumerate() {
        let step_type = string(step, "type")?;
        println!("Step {}: {}", index + 1, step_type);
        let result = run_step(port, &root, step, dry_run);
        match result {
            Ok(()) => {
                ok_count += 1;
                println!("  ok");
            }
            Err(err) => {
                failed_count += 1;
                eprintln!("  failed: {err}");
                if !continue_on_error {
                    return Err(err);
                }
            }
        }
    }
    println!(
        "Batch complete: {} ok, {} failed, {} total",
        ok_count,
        failed_count,
        manifest.steps.len()
    );
    Ok(())
}

fn run_step(port: u16, root: &std::path::Path, step: &Value, dry_run: bool) -> AppResult<()> {
    match string(step, "type")?.as_str() {
        "import-asset" => {
            if dry_run {
                println!("  would import asset {}", string(step, "file")?);
                return Ok(());
            }
            crate::cli::import_asset::run(
                port,
                opt_string(step, "studio"),
                root.join(string(step, "file")?),
                opt_string(step, "to").unwrap_or_else(|| "Workspace".to_string()),
                opt_string(step, "name"),
                number(step, "scale").unwrap_or(1.0) as f32,
                bool_value(step, "anchored").unwrap_or(false),
                !bool_value(step, "noWeld").unwrap_or(false),
                opt_string(step, "textureRoot").map(|value| root.join(value)),
            )
        }
        "import-image" => {
            if dry_run {
                println!("  would import image {}", string(step, "file")?);
                return Ok(());
            }
            crate::cli::import_image::run(
                port,
                opt_string(step, "studio"),
                root.join(string(step, "file")?),
                opt_string(step, "to").unwrap_or_else(|| "StarterGui".to_string()),
                opt_string(step, "name"),
                opt_string(step, "kind").unwrap_or_else(|| "image".to_string()),
                opt_string(step, "size"),
                opt_string(step, "position").unwrap_or_else(|| "0,0".to_string()),
            )
        }
        "import-ui-pack" => {
            if dry_run {
                println!("  would import UI pack");
                return Ok(());
            }
            crate::cli::import_ui_pack::run(
                port,
                opt_string(step, "studio"),
                opt_string(step, "folder").map(|value| root.join(value)),
                opt_string(step, "manifest").map(|value| root.join(value)),
                opt_string(step, "to"),
                opt_string(step, "name"),
                opt_string(step, "kind").unwrap_or_else(|| "image".to_string()),
                false,
            )
        }
        "import-audio" => {
            if dry_run {
                println!("  would import audio");
                return Ok(());
            }
            crate::cli::import_audio::run(
                port,
                opt_string(step, "studio"),
                opt_string(step, "file").map(|value| root.join(value)),
                opt_string(step, "manifest").map(|value| root.join(value)),
                opt_string(step, "to"),
                opt_string(step, "name"),
                opt_string(step, "assetId"),
                number(step, "volume").map(|value| value as f32),
                number(step, "playbackSpeed").map(|value| value as f32),
                bool_value(step, "looped").unwrap_or(false),
                false,
            )
        }
        "validate" => crate::cli::validate::run(
            port,
            opt_string(step, "studio"),
            string(step, "path")?,
            string_list(step, "rules"),
            false,
            bool_value(step, "fix").unwrap_or(false),
        ),
        "repair-tool" | "wire-tool" => crate::cli::repair_tool::run(
            port,
            opt_string(step, "studio"),
            string(step, "path")?,
            opt_string(step, "handle"),
            dry_run || bool_value(step, "dryRun").unwrap_or(false),
            bool_value(step, "replaceBroken").unwrap_or(false),
            bool_value(step, "noPhysicsFix").unwrap_or(false),
            bool_value(step, "collision"),
            bool_value(step, "massless"),
            false,
        ),
        "snapshot" => crate::cli::snapshot::run(
            port,
            opt_string(step, "studio"),
            string(step, "path")?,
            bool_value(step, "includePaths").unwrap_or(false),
            false,
            opt_string(step, "out").map(|value| root.join(value)),
        ),
        "create" => {
            if dry_run {
                println!("  would create {}", string(step, "className")?);
                return Ok(());
            }
            let properties = step
                .get("properties")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            crate::cli::create::run(
                port,
                opt_string(step, "studio"),
                opt_string(step, "className"),
                opt_string(step, "to"),
                opt_string(step, "name"),
                properties,
                None,
                false,
            )
        }
        other => Err(AppError::Other(format!(
            "unsupported batch step type: {other}"
        ))),
    }
}

fn string(value: &Value, key: &str) -> AppResult<String> {
    opt_string(value, key).ok_or_else(|| AppError::Other(format!("step missing {key}")))
}

fn opt_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn bool_value(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn string_list(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
