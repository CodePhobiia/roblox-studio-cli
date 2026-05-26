use crate::error::{AppError, AppResult};
use crate::protocol::messages::ApplyPlanRequest;
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
    let response: Value = crate::cli::request::post(
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
        let verb = if bool_field(&response, "dryRun") {
            "Planned"
        } else {
            "Applied"
        };
        println!(
            "{verb} {} operation(s) under {}",
            usize_field(&response, "applied"),
            string_field(&response, "rootPath")
        );
        println!(
            "Skipped: {}  Refused: {}",
            usize_field(&response, "skipped"),
            usize_field(&response, "refused")
        );
        if let Some(snapshot_recorded) = response.get("snapshotRecorded").and_then(Value::as_bool) {
            println!(
                "Snapshot recorded: {}  Rolled back: {}",
                snapshot_recorded,
                bool_field(&response, "rolledBack")
            );
        }
        print_string_list("Changed paths", array_field(&response, "changedPaths"), 25);
        print_string_list("Refused paths", array_field(&response, "refusedPaths"), 25);
        print_operation_results(&response);
        print_string_list("Warnings", array_field(&response, "warnings"), 25);
    }
    Ok(())
}

fn string_field<'a>(value: &'a Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("<unknown>")
}

fn bool_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn usize_field(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0)
}

fn array_field<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn print_string_list(label: &str, values: &[Value], limit: usize) {
    if values.is_empty() {
        return;
    }
    println!("{label}:");
    for value in values.iter().take(limit) {
        println!("  - {}", value.as_str().unwrap_or("<value>"));
    }
    if values.len() > limit {
        println!("  ... ({} more)", values.len() - limit);
    }
}

fn print_operation_results(response: &Value) {
    let results = array_field(response, "operationResults");
    if results.is_empty() {
        return;
    }
    println!("Operation results:");
    for result in results.iter().take(25) {
        let index = result.get("index").and_then(Value::as_u64).unwrap_or(0);
        let status = result
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("<status>");
        let action = result
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("<action>");
        let path = result
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("<path>");
        if let Some(property) = result.get("property").and_then(Value::as_str) {
            println!("  - #{index} {status} {action} {path}.{property}");
        } else {
            println!("  - #{index} {status} {action} {path}");
        }
        if let Some(message) = result.get("message").and_then(Value::as_str) {
            println!("    {message}");
        }
    }
    if results.len() > 25 {
        println!("  ... ({} more)", results.len() - 25);
    }
}
