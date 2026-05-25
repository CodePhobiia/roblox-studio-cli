use crate::error::AppResult;
use crate::protocol::messages::{DepsRequest, DepsResponse, ValidateRequest, ValidateResponse};
use serde_json::Value;
use std::path::PathBuf;

pub fn run(
    port: u16,
    studio: Option<String>,
    path: String,
    package_path: Option<PathBuf>,
    json: bool,
) -> AppResult<()> {
    let validation: ValidateResponse = crate::cli::request::post(
        port,
        "publish-check validate",
        "/validate",
        &ValidateRequest {
            studio: studio.clone(),
            path: path.clone(),
            rules: vec![
                "refs".into(),
                "welds".into(),
                "tool".into(),
                "assets".into(),
                "paths".into(),
            ],
        },
        120,
    )?;
    let deps: DepsResponse = crate::cli::request::post(
        port,
        "publish-check deps",
        "/deps",
        &DepsRequest {
            studio,
            path: path.clone(),
        },
        120,
    )?;
    let package_report = package_path
        .as_ref()
        .map(|package| crate::cli::package::verify_package_folder(package))
        .transpose()?;

    let mut blockers = Vec::<String>::new();
    let mut warnings = Vec::<String>::new();
    for diagnostic in &validation.diagnostics {
        let line = format!(
            "{} {}{}: {}",
            diagnostic.rule,
            diagnostic.path,
            diagnostic
                .property
                .as_ref()
                .map(|property| format!(".{property}"))
                .unwrap_or_default(),
            diagnostic.message
        );
        if diagnostic.severity == "fail" {
            blockers.push(line);
        } else if diagnostic.severity == "warn" {
            warnings.push(line);
        }
    }
    for dep in &deps.dependencies {
        if dep
            .flags
            .iter()
            .any(|flag| flag == "missing" || flag == "empty")
        {
            blockers.push(format!("missing asset {}.{}", dep.path, dep.property));
        }
        if dep.flags.iter().any(|flag| flag == "privateRisk") {
            warnings.push(format!(
                "asset may be private or inaccessible {}.{} = {}",
                dep.path, dep.property, dep.value
            ));
        }
        if dep.flags.iter().any(|flag| flag == "largeEditableRisk") {
            warnings.push(format!(
                "large editable asset risk {}.{} = {}",
                dep.path, dep.property, dep.value
            ));
        }
    }
    if !deps.unowned_instances.is_empty() {
        warnings.push(format!(
            "{} unowned/manual instance(s) found under managed check path",
            deps.unowned_instances.len()
        ));
    }
    for warning in &deps.warnings {
        warnings.push(warning.clone());
    }
    if let Some(report) = &package_report {
        let missing = report["checksums"]["missing"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0);
        let mismatched = report["checksums"]["mismatched"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0);
        if missing > 0 || mismatched > 0 {
            blockers.push(format!(
                "package checksum drift: {missing} missing, {mismatched} mismatched"
            ));
        }
    }

    let report = serde_json::json!({
        "ok": blockers.is_empty(),
        "path": path,
        "protocolChecked": true,
        "validation": validation,
        "deps": deps,
        "package": package_report,
        "blockers": blockers,
        "warnings": warnings
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        if report["ok"].as_bool().unwrap_or(false) {
            println!(
                "Publish check passed for {}",
                report["path"].as_str().unwrap_or("<unknown>")
            );
        } else {
            println!(
                "Publish check failed for {}",
                report["path"].as_str().unwrap_or("<unknown>")
            );
        }
        print_lines("Blockers", report.get("blockers"));
        print_lines("Warnings", report.get("warnings"));
    }
    Ok(())
}

fn print_lines(title: &str, value: Option<&Value>) {
    let items = value.and_then(Value::as_array).cloned().unwrap_or_default();
    println!("{title}: {}", items.len());
    for item in items.iter().take(25) {
        println!("  - {}", item.as_str().unwrap_or("<non-string item>"));
    }
    if items.len() > 25 {
        println!("  ... ({} more)", items.len() - 25);
    }
}
