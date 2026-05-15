use crate::bridge::auto_spawn::ensure_bridge_running;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{Envelope, ExecRequest};
use std::io::Write;
use std::time::Duration;

pub fn run(port: u16, studio: Option<String>, lua: String) -> AppResult<()> {
    ensure_bridge_running(port)?;
    let url = format!("http://127.0.0.1:{port}/exec");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(35))
        .build()?;
    let resp = client
        .post(&url)
        .json(&ExecRequest { studio, lua })
        .send()
        .map_err(|source| AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?;
    let env: Envelope<serde_json::Value> = resp.json()?;
    if !env.ok {
        return Err(AppError::Other(format!(
            "exec failed: {} (code: {})",
            env.error.unwrap_or_default(),
            env.code.unwrap_or_default()
        )));
    }
    print_json(env.data.unwrap_or_else(|| serde_json::json!(null)))?;
    Ok(())
}

fn print_json(value: serde_json::Value) -> AppResult<()> {
    match value {
        serde_json::Value::String(s) => println!("{}", serde_json::to_string(&s)?),
        other => println!("{}", serde_json::to_string_pretty(&other)?),
    }
    std::io::stdout().flush()?;
    Ok(())
}
