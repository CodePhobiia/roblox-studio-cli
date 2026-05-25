use crate::error::AppResult;
use crate::protocol::messages::{RepairToolRequest, RepairToolResponse};
use std::io::Write;

#[allow(clippy::too_many_arguments)]
pub fn run(
    port: u16,
    studio: Option<String>,
    path: String,
    handle: Option<String>,
    dry_run: bool,
    replace_broken: bool,
    no_physics_fix: bool,
    collision: Option<bool>,
    massless: Option<bool>,
    json: bool,
) -> AppResult<()> {
    let response: RepairToolResponse = crate::cli::request::post(
        port,
        "repair-tool",
        "/repair-tool",
        &RepairToolRequest {
            studio,
            path,
            handle,
            dry_run,
            replace_broken,
            physics_fix: !no_physics_fix,
            collision,
            massless,
        },
        75,
    )?;
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "{} {}",
            if response.dry_run {
                "Planned repair for"
            } else {
                "Repaired"
            },
            response.path
        );
        println!("Parts: {}", response.parts_found);
        println!(
            "Valid joints preserved: {}",
            response.valid_joints_preserved
        );
        println!("Broken joints found: {}", response.broken_joints_found);
        println!("Welds created: {}", response.welds_created);
        println!(
            "Physics properties changed: {}",
            response.physics_properties_changed
        );
        if !response.warnings.is_empty() {
            println!("Warnings ({}):", response.warnings.len());
            for warning in &response.warnings {
                println!("  - {warning}");
            }
        }
    }
    std::io::stdout().flush()?;
    Ok(())
}
