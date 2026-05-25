use crate::error::{AppError, AppResult};
use std::fs::OpenOptions;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn ensure_bridge_running(port: u16) -> AppResult<()> {
    let url = format!("http://127.0.0.1:{port}/healthz");
    if probe(&url) {
        return Ok(());
    }

    let log_path = spawn_bridge(port)?;
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if probe(&url) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    Err(AppError::BridgeStartFailed { port, log_path })
}

fn probe(url: &str) -> bool {
    let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(300))
        .build()
    else {
        return false;
    };
    matches!(client.get(url).send(), Ok(resp) if resp.status().is_success())
}

fn spawn_bridge(port: u16) -> AppResult<String> {
    let exe = std::env::current_exe()?;
    let token = crate::bridge::auth::load_or_create_token()?;
    let log_path = default_log_path();
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let stderr = stdout.try_clone()?;

    let mut command = Command::new(&exe);
    command
        .args(["bridge", "serve", "--port", &port.to_string()])
        .env(crate::bridge::auth::TOKEN_ENV, token)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        command.creation_flags(
            CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS | CREATE_BREAKAWAY_FROM_JOB,
        );
    }

    command.spawn()?;
    Ok(log_path)
}

fn default_log_path() -> String {
    std::env::var_os("USERPROFILE")
        .map(|home| std::path::PathBuf::from(home).join(".rs-bridge.log"))
        .unwrap_or_else(|| std::path::PathBuf::from(".rs-bridge.log"))
        .display()
        .to_string()
}
