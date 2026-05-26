use crate::error::AppResult;
use crate::protocol::messages::{DependencyReference, DepsRequest, DepsResponse};
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
            println!("  - {}", dependency_line(dep));
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

fn dependency_line(dep: &DependencyReference) -> String {
    let mut annotations = Vec::new();
    if !dep.flags.is_empty() {
        annotations.push(format!("flags:{}", dep.flags.join(",")));
    }
    if !dep.rule_ids.is_empty() {
        annotations.push(format!("rules:{}", dep.rule_ids.join(",")));
    }
    let annotations = if annotations.is_empty() {
        String::new()
    } else {
        format!(" [{}]", annotations.join(" "))
    };
    format!(
        "{}{} {}.{} = {}",
        dep.kind, annotations, dep.path, dep.property, dep.value
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_contract_deps_text_output_includes_rule_ids() {
        let dep = DependencyReference {
            path: "Workspace.Tool.Handle".to_string(),
            class_name: "MeshPart".to_string(),
            property: "MeshId".to_string(),
            kind: "mesh".to_string(),
            value: "".to_string(),
            flags: vec!["missing".to_string()],
            rule_ids: vec!["asset.mesh.missing".to_string()],
        };

        let line = dependency_line(&dep);
        assert!(line.contains("rules:asset.mesh.missing"));
        assert!(line.contains("flags:missing"));
    }
}
