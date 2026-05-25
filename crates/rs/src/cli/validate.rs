use crate::error::AppResult;
use crate::protocol::messages::{
    RepairToolRequest, RepairToolResponse, ValidateRequest, ValidateResponse,
};
use serde::Serialize;
use std::io::Write;

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
        let needs_repair = response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.fix_id.as_deref() == Some("repair-tool"));
        let repair = if needs_repair {
            Some(crate::cli::request::post::<_, RepairToolResponse>(
                port,
                "repair-tool",
                "/repair-tool",
                &RepairToolRequest {
                    studio: studio.clone(),
                    path: path.clone(),
                    handle: None,
                    dry_run: false,
                    replace_broken: true,
                    physics_fix: true,
                    collision: Some(false),
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
                Some(repair) => println!(
                    "Applied repair-tool: {} weld(s), {} physics change(s), {} broken joint(s)",
                    repair.welds_created,
                    repair.physics_properties_changed,
                    repair.broken_joints_found
                ),
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
        let property = diagnostic
            .property
            .as_ref()
            .map(|value| format!(" {value}"))
            .unwrap_or_default();
        println!(
            "{}  {}{} {}",
            diagnostic.severity.to_ascii_uppercase(),
            diagnostic.path,
            property,
            diagnostic.message
        );
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
