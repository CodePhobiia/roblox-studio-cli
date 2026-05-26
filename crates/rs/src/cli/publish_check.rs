use crate::error::AppResult;
use crate::protocol::messages::{
    DependencyReference, DepsRequest, DepsResponse, ValidateRequest, ValidateResponse,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishCheckItem {
    id: String,
    severity: String,
    source: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    property: Option<String>,
    message: String,
}

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

    let mut checks = publish_checks(&validation, &deps);
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
            checks.push(PublishCheckItem {
                id: "package.checksum.drift".to_string(),
                severity: "fail".to_string(),
                source: "package".to_string(),
                path: package_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<package>".to_string()),
                property: None,
                message: format!(
                    "package checksum drift: {missing} missing, {mismatched} mismatched"
                ),
            });
        }
    }
    let blockers = lines_for_severity(&checks, "fail");
    let warnings = lines_for_severity(&checks, "warn");

    let report = serde_json::json!({
        "ok": blockers.is_empty(),
        "path": path,
        "protocolChecked": true,
        "validation": validation,
        "deps": deps,
        "package": package_report,
        "checks": checks,
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

fn publish_checks(validation: &ValidateResponse, deps: &DepsResponse) -> Vec<PublishCheckItem> {
    let mut checks = Vec::new();
    for diagnostic in &validation.diagnostics {
        if diagnostic.severity == "fail" || diagnostic.severity == "warn" {
            checks.push(PublishCheckItem {
                id: diagnostic.rule.clone(),
                severity: diagnostic.severity.clone(),
                source: "validate".to_string(),
                path: diagnostic.path.clone(),
                property: diagnostic.property.clone(),
                message: diagnostic.message.clone(),
            });
        }
    }
    for dep in &deps.dependencies {
        checks.extend(dependency_checks(dep));
    }
    for path in &deps.unowned_instances {
        checks.push(PublishCheckItem {
            id: "ownership.unowned".to_string(),
            severity: "warn".to_string(),
            source: "deps".to_string(),
            path: path.clone(),
            property: None,
            message: "instance is not marked as rs-owned".to_string(),
        });
    }
    for warning in &deps.warnings {
        checks.push(PublishCheckItem {
            id: "deps.warning".to_string(),
            severity: "warn".to_string(),
            source: "deps".to_string(),
            path: deps.root_path.clone(),
            property: None,
            message: warning.clone(),
        });
    }
    checks
}

fn dependency_checks(dep: &DependencyReference) -> Vec<PublishCheckItem> {
    let mut checks = Vec::new();
    let ids = dependency_rule_ids(dep);
    if dep
        .flags
        .iter()
        .any(|flag| flag == "missing" || flag == "empty")
    {
        checks.push(PublishCheckItem {
            id: ids
                .iter()
                .find(|id| id.contains(".missing"))
                .cloned()
                .unwrap_or_else(|| missing_rule_for_kind(&dep.kind).to_string()),
            severity: "fail".to_string(),
            source: "deps".to_string(),
            path: dep.path.clone(),
            property: Some(dep.property.clone()),
            message: format!("missing {} asset reference", dep.kind),
        });
    }
    if dep.flags.iter().any(|flag| flag == "privateRisk") {
        checks.push(PublishCheckItem {
            id: "asset.private-risk".to_string(),
            severity: "warn".to_string(),
            source: "deps".to_string(),
            path: dep.path.clone(),
            property: Some(dep.property.clone()),
            message: format!("asset may be private or inaccessible: {}", dep.value),
        });
    }
    if dep.flags.iter().any(|flag| flag == "largeEditableRisk") {
        checks.push(PublishCheckItem {
            id: "asset.editable.large-risk".to_string(),
            severity: "warn".to_string(),
            source: "deps".to_string(),
            path: dep.path.clone(),
            property: Some(dep.property.clone()),
            message: format!("large editable asset risk: {}", dep.value),
        });
    }
    checks
}

fn dependency_rule_ids(dep: &DependencyReference) -> BTreeSet<String> {
    let mut ids = dep.rule_ids.iter().cloned().collect::<BTreeSet<_>>();
    for flag in &dep.flags {
        match flag.as_str() {
            "missing" | "empty" => {
                ids.insert(missing_rule_for_kind(&dep.kind).to_string());
            }
            "privateRisk" => {
                ids.insert("asset.private-risk".to_string());
            }
            "unowned" => {
                ids.insert("ownership.unowned".to_string());
            }
            "largeEditableRisk" => {
                ids.insert("asset.editable.large-risk".to_string());
            }
            _ => {}
        }
    }
    ids
}

fn missing_rule_for_kind(kind: &str) -> &'static str {
    match kind {
        "animation" => "asset.animation.missing",
        "audio" => "asset.sound.missing",
        "mesh" => "asset.mesh.missing",
        _ => "asset.image.missing",
    }
}

fn lines_for_severity(checks: &[PublishCheckItem], severity: &str) -> Vec<String> {
    checks
        .iter()
        .filter(|check| check.severity == severity)
        .map(check_line)
        .collect()
}

fn check_line(check: &PublishCheckItem) -> String {
    let property = check
        .property
        .as_ref()
        .map(|property| format!(".{property}"))
        .unwrap_or_default();
    format!(
        "[{}] {}{}: {}",
        check.id, check.path, property, check.message
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::messages::{Diagnostic, ValidationSummary};

    #[test]
    fn diagnostic_contract_publish_check_preserves_validate_and_deps_ids() {
        let validation = ValidateResponse {
            path: "Workspace.Tool".to_string(),
            summary: ValidationSummary {
                fail: 1,
                warn: 0,
                info: 0,
            },
            diagnostics: vec![Diagnostic {
                severity: "fail".to_string(),
                rule: "tool.part.disconnected".to_string(),
                path: "Workspace.Tool.Part".to_string(),
                property: None,
                message: "BasePart is not rigidly connected to Handle".to_string(),
                fix_id: Some("fix.tool.weld-disconnected-parts".to_string()),
            }],
            warnings: vec![],
        };
        let deps = DepsResponse {
            root_path: "Workspace.Tool".to_string(),
            dependencies: vec![DependencyReference {
                path: "Workspace.Tool.Handle".to_string(),
                class_name: "MeshPart".to_string(),
                property: "MeshId".to_string(),
                kind: "mesh".to_string(),
                value: "".to_string(),
                flags: vec!["missing".to_string()],
                rule_ids: vec!["asset.mesh.missing".to_string()],
            }],
            scripts: vec![],
            remotes: vec![],
            unowned_instances: vec![],
            warnings: vec![],
        };

        let checks = publish_checks(&validation, &deps);
        assert!(checks
            .iter()
            .any(|check| check.id == "tool.part.disconnected"));
        assert!(checks.iter().any(|check| check.id == "asset.mesh.missing"));
        assert!(lines_for_severity(&checks, "fail")
            .iter()
            .any(|line| line.contains("[asset.mesh.missing]")));
    }
}
