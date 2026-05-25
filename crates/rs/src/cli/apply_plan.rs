use crate::error::{AppError, AppResult};
use crate::protocol::messages::{ApplyPlanRequest, ApplyPlanResponse};
use serde_json::Value;
use std::path::PathBuf;

#[allow(clippy::too_many_arguments)]
pub fn run(
    port: u16,
    studio: Option<String>,
    root_path: String,
    file: PathBuf,
    dry_run: bool,
    yes: bool,
    only: Vec<String>,
    exclude: Vec<String>,
    force: bool,
    json: bool,
) -> AppResult<()> {
    if !dry_run && !yes {
        return Err(AppError::Other(
            "apply-plan mutates Studio; pass --dry-run to preview or --yes to apply".into(),
        ));
    }
    let plan: Value = serde_json::from_str(&std::fs::read_to_string(&file)?)?;
    let response: ApplyPlanResponse = crate::cli::request::post(
        port,
        "apply-plan",
        "/apply-plan",
        &ApplyPlanRequest {
            studio,
            root_path,
            plan,
            dry_run,
            approved: yes,
            force,
            only,
            exclude,
        },
        210,
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        let verb = if response.dry_run {
            "Planned"
        } else {
            "Applied"
        };
        println!(
            "{verb} {} operation(s) under {}",
            response.applied, response.root_path
        );
        println!(
            "Skipped: {}  Refused: {}",
            response.skipped, response.refused
        );
        if !response.changed_paths.is_empty() {
            println!("Changed paths:");
            for path in response.changed_paths.iter().take(25) {
                println!("  - {path}");
            }
            if response.changed_paths.len() > 25 {
                println!("  ... ({} more)", response.changed_paths.len() - 25);
            }
        }
        if !response.warnings.is_empty() {
            println!("Warnings:");
            for warning in response.warnings.iter().take(25) {
                println!("  - {warning}");
            }
            if response.warnings.len() > 25 {
                println!("  ... ({} more)", response.warnings.len() - 25);
            }
        }
    }
    Ok(())
}
