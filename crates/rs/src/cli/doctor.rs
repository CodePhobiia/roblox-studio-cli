use crate::bridge::auto_spawn::ensure_bridge_running;
use crate::error::{AppError, AppResult};
use crate::protocol::messages::{Envelope, StudioInfo, CLI_VERSION, PLUGIN_PROTOCOL_VERSION};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoctorReport {
    ok: bool,
    expected_protocol_version: u32,
    cli_version: String,
    bridge: Check,
    plugin_install: Check,
    studios: Vec<StudioCheck>,
    fixes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Check {
    status: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StudioCheck {
    id: String,
    name: String,
    status: String,
    message: String,
    protocol_version: Option<u32>,
    plugin_version: Option<String>,
    capabilities: Vec<String>,
    last_heartbeat_ms_ago: u64,
}

pub fn run(port: u16, fix: bool, json: bool) -> AppResult<()> {
    let mut fixes = Vec::new();
    let bridge_running = probe_bridge(port);
    if fix && !bridge_running {
        if let Err(err) = ensure_bridge_running(port) {
            if !probe_bridge(port) {
                fixes.push(format!(
                    "Could not start the bridge automatically: {err}. Run `rs bridge serve` in another terminal and retry."
                ));
            }
        }
    }
    let bridge_running = probe_bridge(port);
    let studios = if bridge_running {
        fetch_studios(port)?
    } else {
        Vec::new()
    };

    let plugin_path = plugin_install_path();
    let repo_plugin = repo_plugin_path();
    let plugin_check = check_plugin_install(&plugin_path, repo_plugin.as_deref(), fix, &mut fixes)?;
    let studio_checks = studios.into_iter().map(check_studio).collect::<Vec<_>>();

    if studio_checks
        .iter()
        .any(|studio| studio.protocol_version != Some(PLUGIN_PROTOCOL_VERSION))
    {
        fixes.push(
            "Restart every open Roblox Studio window after installing the rebuilt plugin.".into(),
        );
    }
    if studio_checks.is_empty() {
        fixes.push(
            "Open a Roblox Studio place with the rs plugin installed, then run `rs doctor` again."
                .into(),
        );
    }

    let bridge = if bridge_running {
        Check {
            status: "pass".into(),
            message: format!("bridge running on 127.0.0.1:{port}"),
        }
    } else {
        Check {
            status: "fail".into(),
            message: format!("bridge not reachable on 127.0.0.1:{port}"),
        }
    };

    let ok = bridge.status == "pass"
        && plugin_check.status != "fail"
        && !studio_checks.is_empty()
        && !studio_checks.iter().any(|studio| studio.status == "fail");
    let report = DoctorReport {
        ok,
        expected_protocol_version: PLUGIN_PROTOCOL_VERSION,
        cli_version: CLI_VERSION.to_string(),
        bridge,
        plugin_install: plugin_check,
        studios: studio_checks,
        fixes,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }
    if report.ok {
        Ok(())
    } else if json {
        Err(AppError::Silent {
            exit_code: 1,
            message: "doctor found issues".into(),
        })
    } else {
        Err(AppError::Other(
            "doctor found issues; see FIX lines above".into(),
        ))
    }
}

fn probe_bridge(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/healthz");
    let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    else {
        return false;
    };
    matches!(client.get(url).send(), Ok(resp) if resp.status().is_success())
}

fn fetch_studios(port: u16) -> AppResult<Vec<StudioInfo>> {
    let url = format!("http://127.0.0.1:{port}/studios");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;
    let env: Envelope<Vec<StudioInfo>> = client
        .get(&url)
        .send()
        .map_err(|source| AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?
        .json()?;
    if !env.ok {
        return Err(crate::cli::envelope_error("doctor", env.error, env.code));
    }
    Ok(env.data.unwrap_or_default())
}

fn check_plugin_install(
    plugin_path: &Path,
    repo_plugin: Option<&Path>,
    fix: bool,
    fixes: &mut Vec<String>,
) -> AppResult<Check> {
    if fix {
        if let Some(repo_plugin) = repo_plugin {
            if repo_plugin.exists() {
                if let Some(parent) = plugin_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(repo_plugin, plugin_path)?;
            }
        }
    }

    if !plugin_path.exists() {
        fixes.push(format!(
            "Build and copy plugin/rs-bridge-plugin.rbxmx to {}.",
            plugin_path.display()
        ));
        return Ok(Check {
            status: "fail".into(),
            message: format!("plugin is not installed at {}", plugin_path.display()),
        });
    }

    let installed = modified_time(plugin_path)?;
    let mut message = format!(
        "plugin installed at {} ({})",
        plugin_path.display(),
        format_system_time(installed)
    );
    if let Some(repo_plugin) = repo_plugin.filter(|path| path.exists()) {
        let repo_time = modified_time(repo_plugin)?;
        if installed < repo_time {
            fixes.push(format!(
                "Copy {} to {} and restart Studio.",
                repo_plugin.display(),
                plugin_path.display()
            ));
            return Ok(Check {
                status: "warn".into(),
                message: format!(
                    "{message}; repo bundle is newer ({})",
                    format_system_time(repo_time)
                ),
            });
        }
        message.push_str("; installed bundle is current relative to repo bundle");
    }
    Ok(Check {
        status: "pass".into(),
        message,
    })
}

fn check_studio(studio: StudioInfo) -> StudioCheck {
    let (status, message) = match studio.protocol_version {
        Some(version) if version == PLUGIN_PROTOCOL_VERSION => (
            "pass".to_string(),
            format!("plugin protocol v{version} matches CLI"),
        ),
        Some(version) => (
            "fail".to_string(),
            format!("plugin protocol v{version}, CLI expects v{PLUGIN_PROTOCOL_VERSION}"),
        ),
        None => (
            "fail".to_string(),
            format!("plugin protocol unknown, CLI expects v{PLUGIN_PROTOCOL_VERSION}"),
        ),
    };
    StudioCheck {
        id: studio.id,
        name: studio.name,
        status,
        message,
        protocol_version: studio.protocol_version,
        plugin_version: studio.plugin_version,
        capabilities: studio.capabilities,
        last_heartbeat_ms_ago: studio.last_heartbeat_ms_ago,
    }
}

fn print_report(report: &DoctorReport) {
    println!("rs doctor");
    println!("CLI version: {}", report.cli_version);
    println!(
        "Expected plugin protocol: {}",
        report.expected_protocol_version
    );
    print_check("Bridge", &report.bridge);
    print_check("Plugin", &report.plugin_install);
    if report.studios.is_empty() {
        println!("WARN  Studios: none connected");
    } else {
        for studio in &report.studios {
            println!(
                "{}  Studio {} ({}) - {}",
                studio.status.to_ascii_uppercase(),
                studio.name,
                studio.id,
                studio.message
            );
        }
    }
    for fix in &report.fixes {
        println!("FIX   {fix}");
    }
}

fn print_check(label: &str, check: &Check) {
    println!(
        "{}  {}: {}",
        check.status.to_ascii_uppercase(),
        label,
        check.message
    );
}

fn plugin_install_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Roblox")
        .join("Plugins")
        .join("rs-bridge-plugin.rbxmx")
}

fn repo_plugin_path() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    let candidate = current.join("plugin").join("rs-bridge-plugin.rbxmx");
    candidate.exists().then_some(candidate)
}

fn modified_time(path: &Path) -> AppResult<SystemTime> {
    Ok(std::fs::metadata(path)?.modified()?)
}

fn format_system_time(time: SystemTime) -> String {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => format!("unix {}", duration.as_secs()),
        Err(_) => "before unix epoch".into(),
    }
}
