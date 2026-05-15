use crate::bridge::auto_spawn::ensure_bridge_running;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{Envelope, StudioInfo};
use std::io::Write;
use std::time::Duration;

pub fn run(port: u16, json: bool) -> AppResult<()> {
    ensure_bridge_running(port)?;
    let url = format!("http://127.0.0.1:{port}/studios");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|source| AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?;
    let env: Envelope<Vec<StudioInfo>> = resp.json()?;
    if !env.ok {
        return Err(AppError::Other(format!(
            "list failed: {} (code: {})",
            env.error.unwrap_or_default(),
            env.code.unwrap_or_default()
        )));
    }
    let studios = env.data.unwrap_or_default();
    if json {
        println!("{}", serde_json::to_string_pretty(&studios)?);
    } else if studios.is_empty() {
        println!("No Studios connected.");
    } else {
        println!("{:<36}  {:<28}  {:>10}  Path", "ID", "Name", "Heartbeat");
        for studio in studios {
            println!(
                "{:<36}  {:<28}  {:>7}ms  {}",
                studio.id,
                truncate(&studio.name, 28),
                studio.last_heartbeat_ms_ago,
                studio.place_file_path.as_deref().unwrap_or("")
            );
        }
    }
    std::io::stdout().flush()?;
    Ok(())
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    let mut s: String = value.chars().take(width.saturating_sub(3)).collect();
    s.push_str("...");
    s
}
