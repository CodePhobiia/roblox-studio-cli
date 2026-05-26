use crate::error::AppResult;
use crate::protocol::messages::{
    Diagnostic, RepairToolRequest, RepairToolResponse, ValidateRequest, ValidateResponse,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::io::Write;

const FIX_TOOL_REMOVE_BROKEN_JOINTS: &str = "fix.tool.remove-broken-joints";
const FIX_TOOL_SET_PART_PHYSICS: &str = "fix.tool.set-part-physics";
const FIX_TOOL_UNANCHOR_PARTS: &str = "fix.tool.unanchor-parts";
const FIX_TOOL_WELD_DISCONNECTED_PARTS: &str = "fix.tool.weld-disconnected-parts";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidateFixReport {
    before: ValidateResponse,
    repair: Option<RepairToolResponse>,
    after: ValidateResponse,
}

pub fn run(
    port: u16,
    studio: Option<String>,
    path: String,
    rules: Vec<String>,
    json: bool,
    fix: bool,
) -> AppResult<()> {
    let response = validate_once(port, studio.clone(), path.clone(), rules.clone())?;
    if fix {
        let fix_ids = applicable_validate_fix_ids(&response);
        let repair = if !fix_ids.is_empty() {
            Some(crate::cli::request::post::<_, RepairToolResponse>(
                port,
                "repair-tool",
                "/repair-tool",
                &RepairToolRequest {
                    studio: studio.clone(),
                    path: path.clone(),
                    handle: None,
                    dry_run: false,
                    replace_broken: fix_ids.iter().any(|id| id == FIX_TOOL_REMOVE_BROKEN_JOINTS),
                    physics_fix: fix_ids
                        .iter()
                        .any(|id| id == FIX_TOOL_SET_PART_PHYSICS || id == FIX_TOOL_UNANCHOR_PARTS),
                    collision: fix_ids
                        .iter()
                        .any(|id| id == FIX_TOOL_SET_PART_PHYSICS)
                        .then_some(false),
                    massless: None,
                },
                75,
            )?)
        } else {
            None
        };
        let after = validate_once(port, studio, path, rules)?;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&ValidateFixReport {
                    before: response,
                    repair,
                    after
                })?
            );
        } else {
            println!("Before:");
            print_response(&response);
            match &repair {
                Some(repair) => {
                    let fix_summary = if repair.fix_ids.is_empty() {
                        "none reported".to_string()
                    } else {
                        repair.fix_ids.join(",")
                    };
                    println!(
                        "Applied repair-tool fixes [{}]: {} weld(s), {} physics change(s), {} broken joint(s)",
                        fix_summary,
                        repair.welds_created,
                        repair.physics_properties_changed,
                        repair.broken_joints_found
                    );
                }
                None => println!("No safe fixes were applicable."),
            }
            println!("After:");
            print_response(&after);
        }
        std::io::stdout().flush()?;
        return Ok(());
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        print_response(&response);
    }
    std::io::stdout().flush()?;
    Ok(())
}

fn validate_once(
    port: u16,
    studio: Option<String>,
    path: String,
    rules: Vec<String>,
) -> AppResult<ValidateResponse> {
    crate::cli::request::post(
        port,
        "validate",
        "/validate",
        &ValidateRequest {
            studio,
            path,
            rules,
        },
        75,
    )
}

fn print_response(response: &ValidateResponse) {
    for diagnostic in &response.diagnostics {
        println!("{}", diagnostic_line(diagnostic));
    }
    if !response.warnings.is_empty() {
        println!("Warnings ({}):", response.warnings.len());
        for warning in &response.warnings {
            println!("  - {warning}");
        }
    }
    println!(
        "{} fail, {} warn, {} info",
        response.summary.fail, response.summary.warn, response.summary.info
    );
}

fn diagnostic_line(diagnostic: &Diagnostic) -> String {
    let property = diagnostic
        .property
        .as_ref()
        .map(|value| format!(" {value}"))
        .unwrap_or_default();
    let fix = diagnostic
        .fix_id
        .as_ref()
        .map(|value| format!(" fix:{value}"))
        .unwrap_or_default();
    format!(
        "{}  [{}{}] {}{} {}",
        diagnostic.severity.to_ascii_uppercase(),
        diagnostic.rule,
        fix,
        diagnostic.path,
        property,
        diagnostic.message
    )
}

fn applicable_validate_fix_ids(response: &ValidateResponse) -> Vec<String> {
    response
        .diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.fix_id.as_deref())
        .filter(|fix_id| is_safe_validate_fix_id(fix_id))
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_safe_validate_fix_id(fix_id: &str) -> bool {
    matches!(
        fix_id,
        FIX_TOOL_REMOVE_BROKEN_JOINTS
            | FIX_TOOL_SET_PART_PHYSICS
            | FIX_TOOL_UNANCHOR_PARTS
            | FIX_TOOL_WELD_DISCONNECTED_PARTS
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::messages::{ValidateResponse, ValidationSummary};

    #[test]
    fn diagnostic_contract_validate_text_output_includes_rule_and_fix_ids() {
        let diagnostic = Diagnostic {
            severity: "fail".to_string(),
            rule: "tool.part.disconnected".to_string(),
            path: "Workspace.Tool.Part".to_string(),
            property: None,
            message: "BasePart is not rigidly connected to Handle".to_string(),
            fix_id: Some("fix.tool.weld-disconnected-parts".to_string()),
        };

        let line = diagnostic_line(&diagnostic);
        assert!(line.contains("[tool.part.disconnected fix:fix.tool.weld-disconnected-parts]"));
        assert!(line.contains("Workspace.Tool.Part"));
    }

    #[test]
    fn diagnostic_contract_validate_fix_filters_legacy_and_non_fixable_ids() {
        let response = ValidateResponse {
            path: "Workspace.Tool".to_string(),
            summary: ValidationSummary::default(),
            diagnostics: vec![
                Diagnostic {
                    severity: "fail".to_string(),
                    rule: "tool.handle.missing".to_string(),
                    path: "Workspace.Tool".to_string(),
                    property: Some("Handle".to_string()),
                    message: "Tool requires a BasePart child named Handle".to_string(),
                    fix_id: Some("repair-tool".to_string()),
                },
                Diagnostic {
                    severity: "fail".to_string(),
                    rule: "tool.part.disconnected".to_string(),
                    path: "Workspace.Tool.Part".to_string(),
                    property: None,
                    message: "BasePart is not rigidly connected to Handle".to_string(),
                    fix_id: Some("fix.tool.weld-disconnected-parts".to_string()),
                },
            ],
            warnings: vec![],
        };

        assert_eq!(
            applicable_validate_fix_ids(&response),
            vec!["fix.tool.weld-disconnected-parts".to_string()]
        );
    }
}
