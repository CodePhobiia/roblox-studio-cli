use crate::error::{AppError, AppResult};
use crate::protocol::messages::{DeserializeRequest, SerializeRequest, TransferRequest};
use std::io::Write;

pub fn run(
    port: u16,
    from: String,
    to: String,
    dry_run: bool,
    replace: bool,
    rollback_on_error: bool,
    allow_external_refs: bool,
    image_rehost: Option<crate::cli::rehost_images::ImageRehostOptions>,
) -> AppResult<()> {
    let (from_studio, from_path) = parse_studio_path(&from)?;
    let (to_studio, to_parent_path) = parse_studio_path(&to)?;
    if let Some(mut image_rehost) = image_rehost {
        image_rehost.dry_run = dry_run;
        if dry_run {
            println!("Planning transfer {from} -> {to}...");
        } else {
            println!("Transferring {from} -> {to}...");
        }
        let mut blob: serde_json::Value = crate::cli::request::post(
            port,
            "transfer serialize",
            "/serialize",
            &SerializeRequest {
                studio: Some(from_studio),
                path: from_path,
            },
            180,
        )?;
        let image_rehost_report =
            crate::cli::rehost_images::rehost_image_refs_in_blob(&mut blob, &image_rehost)?;
        let data: serde_json::Value = crate::cli::request::post(
            port,
            "transfer deserialize",
            "/deserialize",
            &DeserializeRequest {
                studio: Some(to_studio),
                parent_path: to_parent_path,
                blob,
                conflict_mode: replace.then_some("replace".to_string()),
                dry_run,
                rollback_on_error,
                package_id: None,
                validate_rules: transfer_validation_rules(),
                fail_on_validation_failure: true,
                fail_on_external_refs: !allow_external_refs,
            },
            210,
        )?;
        print_transfer_result(dry_run, &data);
        crate::cli::rehost_images::print_rehost_summary(&image_rehost_report);
        print_warnings(&data);
        std::io::stdout().flush()?;
        return Ok(());
    }

    if dry_run {
        println!("Planning transfer {from} -> {to}...");
    } else {
        println!("Transferring {from} -> {to}...");
    }
    let data: serde_json::Value = crate::cli::request::post(
        port,
        "transfer",
        "/transfer",
        &TransferRequest {
            from_studio,
            from_path,
            to_studio,
            to_parent_path,
            conflict_mode: replace.then_some("replace".to_string()),
            dry_run,
            rollback_on_error,
            allow_external_refs,
        },
        180,
    )?;

    print_transfer_result(dry_run, &data);
    print_warnings(&data);
    std::io::stdout().flush()?;
    Ok(())
}

fn transfer_validation_rules() -> Vec<String> {
    ["refs", "welds", "tool"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn print_transfer_result(dry_run: bool, data: &serde_json::Value) {
    if dry_run {
        println!("OK: dry-run complete");
    } else if let Some(root_path) = data.get("rootPath").and_then(|v| v.as_str()) {
        println!("OK: created at {root_path}");
    } else {
        println!("OK");
    }
}

fn print_warnings(data: &serde_json::Value) {
    if let Some(warnings) = data.get("warnings").and_then(|v| v.as_array()) {
        if !warnings.is_empty() {
            println!("Warnings ({}):", warnings.len());
            for warning in warnings.iter().take(20) {
                println!("  - {}", warning.as_str().unwrap_or("<non-string warning>"));
            }
            if warnings.len() > 20 {
                println!("  ... ({} more)", warnings.len() - 20);
            }
        }
    }
}

fn parse_studio_path(input: &str) -> AppResult<(String, String)> {
    let (studio, path) = input
        .split_once(':')
        .ok_or_else(|| AppError::Other(format!("expected 'studio:path', got '{input}'")))?;
    if studio.trim().is_empty() || path.trim().is_empty() {
        return Err(AppError::Other(format!(
            "expected 'studio:path', got '{input}'"
        )));
    }
    Ok((studio.to_string(), path.to_string()))
}
