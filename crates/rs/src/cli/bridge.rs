use crate::error::{AppError, AppResult};
use crate::protocol::messages::{Envelope, StudioInfo};
use std::io::Write;
use std::time::{Duration, Instant};

pub fn status(port: u16, json: bool) -> AppResult<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    let health_url = format!("http://127.0.0.1:{port}/healthz");
    match client.get(&health_url).send() {
        Ok(resp) if resp.status().is_success() => {}
        Ok(resp) => {
            return Err(AppError::Other(format!(
                "bridge status failed with HTTP {}",
                resp.status()
            )))
        }
        Err(source) => {
            return Err(AppError::BridgeUnreachable {
                url: health_url,
                source,
            })
        }
    }

    let studios_url = format!("http://127.0.0.1:{port}/studios");
    let env: Envelope<Vec<StudioInfo>> = client.get(&studios_url).send()?.json()?;
    if !env.ok {
        return Err(crate::cli::envelope_error(
            "bridge status",
            env.error,
            env.code,
        ));
    }
    let studios = env.data.unwrap_or_default();
    if json {
        println!("{}", serde_json::to_string_pretty(&studios)?);
    } else {
        println!("Bridge running on 127.0.0.1:{port}");
        print_studios(&studios);
    }
    std::io::stdout().flush()?;
    Ok(())
}

pub fn stop(port: u16) -> AppResult<()> {
    let url = format!("http://127.0.0.1:{port}/shutdown");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    let resp = client
        .post(&url)
        .send()
        .map_err(|source| AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?;
    if !resp.status().is_success() {
        return Err(AppError::Other(format!(
            "bridge stop failed with HTTP {}",
            resp.status()
        )));
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let health_url = format!("http://127.0.0.1:{port}/healthz");
    while Instant::now() < deadline {
        match client.get(&health_url).send() {
            Ok(resp) if resp.status().is_success() => {
                std::thread::sleep(Duration::from_millis(100));
            }
            _ => {
                println!("Bridge stopped.");
                std::io::stdout().flush()?;
                return Ok(());
            }
        }
    }

    println!("Bridge stopping; port {port} is still responding.");
    std::io::stdout().flush()?;
    Ok(())
}

fn print_studios(studios: &[StudioInfo]) {
    if studios.is_empty() {
        println!("No Studios connected.");
        return;
    }
    println!(
        "{:<36}  {:<28}  {:>8}  {:>10}  Path",
        "ID", "Name", "Proto", "Heartbeat"
    );
    for studio in studios {
        println!(
            "{:<36}  {:<28}  {:>8}  {:>7}ms  {}",
            studio.id,
            truncate(&studio.name, 28),
            studio
                .protocol_version
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".into()),
            studio.last_heartbeat_ms_ago,
            studio.place_file_path.as_deref().unwrap_or("")
        );
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_string();
    }
    let mut s: String = value.chars().take(width.saturating_sub(3)).collect();
    s.push_str("...");
    s
}
