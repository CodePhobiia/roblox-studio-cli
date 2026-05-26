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
        "export" => {
            if dry_run {
                println!("  would export {}", string(step, "path")?);
                return Ok(());
            }
            crate::cli::export::run(
                port,
                opt_string(step, "studio"),
                string(step, "path")?,
                batch_path(root, step, "out")?,
                number(step, "depth").map(|value| value as u32),
                bool_value(step, "overwrite").unwrap_or(false),
            )
        }
        "diff" => {
            if dry_run {
                println!("  would diff sources");
                return Ok(());
            }
            crate::cli::diff::run(
                port,
                opt_string(step, "studio"),
                opt_string(step, "path"),
                opt_path(root, step, "export")?,
                opt_string(step, "againstStudio"),
                opt_string(step, "againstPath"),
                opt_path(root, step, "againstExport")?,
                number(step, "depth")
                    .map(|value| value as u32)
                    .unwrap_or(999),
                bool_value(step, "json").unwrap_or(false),
                bool_value(step, "ignoreScripts").unwrap_or(false),
                bool_value(step, "ignoreAssets").unwrap_or(false),
                bool_value(step, "fixPlan").unwrap_or(false),
            )
        }
        "apply-plan" => crate::cli::apply_plan::run(
            port,
            opt_string(step, "studio"),
            string(step, "root")?,
            batch_path(root, step, "file")?,
            dry_run || bool_value(step, "dryRun").unwrap_or(false),
            bool_value(step, "yes").unwrap_or(false),
            string_list(step, "only"),
            string_list(step, "exclude"),
            bool_value(step, "force").unwrap_or(false),
            bool_value(step, "json").unwrap_or(false),
        ),
        "sync-folder" => crate::cli::sync_folder::run(
            port,
            opt_string(step, "studio"),
            opt_path(root, step, "folder")?,
            opt_string(step, "to"),
            opt_path(root, step, "manifest")?,
            bool_value(step, "watch").unwrap_or(false),
            dry_run || bool_value(step, "dryRun").unwrap_or(false),
            bool_value(step, "delete").unwrap_or(false),
            bool_value(step, "force").unwrap_or(false),
        ),
        "sync-pull" | "sync pull" => {
            if dry_run {
                println!("  would sync pull {}", string(step, "path")?);
                return Ok(());
            }
            crate::cli::sync_pull::run(
                port,
                opt_string(step, "studio"),
                string(step, "path")?,
                batch_path(root, step, "out")?,
                number(step, "depth").map(|value| value as u32),
                bool_value(step, "overwrite").unwrap_or(false),
                bool_value(step, "json").unwrap_or(false),
            )
        }
        "package-verify" | "package verify" => crate::cli::package::verify_run(
            port,
            opt_string(step, "studio"),
            batch_path(root, step, "file")?,
            opt_string(step, "to"),
            opt_string(step, "ifExists").unwrap_or_else(|| "fail".to_string()),
            bool_value(step, "json").unwrap_or(false),
        ),
        "package-update" | "package update" => {
            if dry_run {
                println!("  would package update {}", string(step, "file")?);
                return Ok(());
            }
            crate::cli::package::update_run(
                port,
                opt_string(step, "studio"),
                batch_path(root, step, "file")?,
                opt_string(step, "to").unwrap_or_else(|| "Workspace".to_string()),
                crate::cli::package::PackageUpdateFlags {
                    owned_only: bool_value(step, "ownedOnly").unwrap_or(false),
                    preserve_local: bool_value(step, "preserveLocal").unwrap_or(false),
                    replace_owned: bool_value(step, "replaceOwned").unwrap_or(false),
                    conflict_report: bool_value(step, "conflictReport").unwrap_or(false),
                    dry_run: dry_run || bool_value(step, "dryRun").unwrap_or(false),
                    force: bool_value(step, "force").unwrap_or(false),
                    json: bool_value(step, "json").unwrap_or(false),
                },
            )
        }
        "transaction-snapshot" | "transaction snapshot" => {
            if dry_run {
                println!(
                    "  would write transaction snapshot {}",
                    string(step, "path")?
                );
                return Ok(());
            }
            crate::cli::transaction::snapshot_run(
                port,
                opt_string(step, "studio"),
                string(step, "path")?,
                batch_path(root, step, "out")?,
                bool_value(step, "json").unwrap_or(false),
            )
        }
        "transaction-restore" | "transaction restore" => {
            if dry_run {
                println!(
                    "  would restore transaction snapshot {}",
                    string(step, "file")?
                );
                return Ok(());
            }
            crate::cli::transaction::restore_run(
                port,
                opt_string(step, "studio"),
                batch_path(root, step, "file")?,
                opt_string(step, "to").unwrap_or_else(|| "Workspace".to_string()),
                opt_string(step, "ifExists").unwrap_or_else(|| "fail".to_string()),
                bool_value(step, "json").unwrap_or(false),
            )
        }
        "history-show" | "history show" => crate::cli::history::show(
            port,
            opt_string(step, "studio"),
            string(step, "id")?,
            bool_value(step, "json").unwrap_or(false),
        ),
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

fn batch_path(root: &std::path::Path, value: &Value, key: &str) -> AppResult<PathBuf> {
    let relative = string(value, key)?;
    crate::cli::export::safe_relative_join(root, &relative, "batch file path")
}

fn opt_path(root: &std::path::Path, value: &Value, key: &str) -> AppResult<Option<PathBuf>> {
    opt_string(value, key)
        .map(|relative| crate::cli::export::safe_relative_join(root, &relative, "batch file path"))
        .transpose()
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

#[cfg(test)]
mod tests {
    use super::{batch_path, run_step};
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn batch_path_rejects_parent_traversal() {
        let step = json!({ "file": "../outside.json" });
        assert!(batch_path(Path::new("root"), &step, "file").is_err());
    }

    #[test]
    fn dry_run_accepts_expanded_step_types_without_live_dispatch() {
        let root = Path::new(".");
        let cases = [
            json!({ "type": "export", "path": "Workspace.Tool", "out": "out" }),
            json!({ "type": "diff", "export": "left", "againstExport": "right" }),
            json!({ "type": "sync-pull", "path": "Workspace.Tool", "out": "out" }),
            json!({ "type": "transaction-snapshot", "path": "Workspace.Tool", "out": "snap.json" }),
            json!({ "type": "transaction-restore", "file": "snap.json", "to": "Workspace" }),
        ];
        for step in cases {
            run_step(0, root, &step, true).unwrap();
        }
    }
}
