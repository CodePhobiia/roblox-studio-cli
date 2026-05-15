use crate::bridge::auto_spawn::ensure_bridge_running;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{Envelope, TransferRequest};
use std::io::Write;
use std::time::Duration;

pub fn run(port: u16, from: String, to: String) -> AppResult<()> {
    ensure_bridge_running(port)?;
    let (from_studio, from_path) = parse_studio_path(&from)?;
    let (to_studio, to_parent_path) = parse_studio_path(&to)?;
    let url = format!("http://127.0.0.1:{port}/transfer");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()?;
    println!("Transferring {from} -> {to}...");
    let resp = client
        .post(&url)
        .json(&TransferRequest {
            from_studio,
            from_path,
            to_studio,
            to_parent_path,
        })
        .send()
        .map_err(|source| AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?;
    let env: Envelope<serde_json::Value> = resp.json()?;
    if !env.ok {
        return Err(crate::cli::envelope_error("transfer", env.error, env.code));
    }

    let data = env.data.unwrap_or_else(|| serde_json::json!({}));
    if let Some(root_path) = data.get("rootPath").and_then(|v| v.as_str()) {
        println!("OK: created at {root_path}");
    } else {
        println!("OK");
    }
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
    std::io::stdout().flush()?;
    Ok(())
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
