use crate::error::AppResult;
use crate::protocol::messages::{DepsRequest, DepsResponse};
use std::path::PathBuf;

pub fn run(
    port: u16,
    studio: Option<String>,
    path: String,
    out: Option<PathBuf>,
    json: bool,
) -> AppResult<()> {
    let response: DepsResponse =
        crate::cli::request::post(port, "deps", "/deps", &DepsRequest { studio, path }, 120)?;
    if let Some(out) = out {
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out, serde_json::to_string_pretty(&response)?)?;
        if !json {
            println!("Dependency graph written to {}", out.display());
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Dependencies for {}", response.root_path);
        println!("Assets: {}", response.dependencies.len());
        println!("Scripts: {}", response.scripts.len());
        println!("Remotes: {}", response.remotes.len());
        println!("Unowned instances: {}", response.unowned_instances.len());
        for dep in response.dependencies.iter().take(30) {
            let flags = if dep.flags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", dep.flags.join(","))
            };
            println!(
                "  - {} {}.{} = {}{}",
                dep.kind, dep.path, dep.property, dep.value, flags
            );
        }
        if response.dependencies.len() > 30 {
            println!("  ... ({} more)", response.dependencies.len() - 30);
        }
        if !response.warnings.is_empty() {
            println!("Warnings:");
            for warning in response.warnings.iter().take(20) {
                println!("  - {warning}");
            }
        }
    }
    Ok(())
}
