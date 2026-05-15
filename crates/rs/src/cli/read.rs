use crate::bridge::auto_spawn::ensure_bridge_running;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{Envelope, ReadRequest};
use std::io::Write;
use std::time::Duration;

pub fn run(port: u16, studio: Option<String>, path: String, depth: u32) -> AppResult<()> {
    ensure_bridge_running(port)?;
    let url = format!("http://127.0.0.1:{port}/read");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(35))
        .build()?;
    let resp = client
        .post(&url)
        .json(&ReadRequest {
            studio,
            path,
            depth,
        })
        .send()
        .map_err(|source| AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?;
    let env: Envelope<serde_json::Value> = resp.json()?;
    if !env.ok {
        return Err(AppError::Other(format!(
            "read failed: {} (code: {})",
            env.error.unwrap_or_default(),
            env.code.unwrap_or_default()
        )));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&env.data.unwrap_or_else(|| serde_json::json!(null)))?
    );
    std::io::stdout().flush()?;
    Ok(())
}
