use crate::error::AppResult;
use crate::protocol::messages::{SnapshotRequest, SnapshotResponse};
use std::io::Write;

pub fn run(
    port: u16,
    studio: Option<String>,
    path: String,
    include_paths: bool,
    json: bool,
    out: Option<std::path::PathBuf>,
) -> AppResult<()> {
    let response: SnapshotResponse = crate::cli::request::post(
        port,
        "snapshot",
        "/snapshot",
        &SnapshotRequest {
            studio,
            path,
            include_paths,
        },
        75,
    )?;

    if let Some(out) = out {
        std::fs::write(&out, serde_json::to_string_pretty(&response)?)?;
        println!(
            "Wrote snapshot for {} to {}",
            response.root_path,
            out.display()
        );
    } else if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!("Snapshot: {}", response.root_path);
        println!("Instances: {}", response.total_instances);
        println!("Max depth: {}", response.max_depth);
        println!("Tools: {}", response.tool_count);
        println!("MeshParts: {}", response.mesh_part_count);
        println!("UI instances: {}", response.ui_count);
        println!("Remotes: {}", response.remote_count);
        print_counts("Classes", &response.class_counts);
        print_counts("Scripts", &response.script_counts);
        if !response.asset_references.is_empty() {
            println!("Asset references: {}", response.asset_references.len());
            for asset in response.asset_references.iter().take(20) {
                println!("  {}.{} -> {}", asset.path, asset.property, asset.asset_uri);
            }
        }
        if !response.duplicate_sibling_names.is_empty() {
            println!(
                "Duplicate sibling names: {}",
                response.duplicate_sibling_names.len()
            );
            for duplicate in response.duplicate_sibling_names.iter().take(20) {
                println!(
                    "  {} has {} children named {}",
                    duplicate.parent_path, duplicate.count, duplicate.name
                );
            }
        }
        if !response.top_subtrees.is_empty() {
            println!("Largest subtrees:");
            for subtree in response.top_subtrees.iter().take(10) {
                println!("  {}: {}", subtree.path, subtree.count);
            }
        }
    }
    std::io::stdout().flush()?;
    Ok(())
}

fn print_counts(label: &str, counts: &std::collections::BTreeMap<String, usize>) {
    if counts.is_empty() {
        return;
    }
    println!("{label}:");
    for (name, count) in counts {
        println!("  {name}: {count}");
    }
}
