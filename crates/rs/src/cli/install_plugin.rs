use crate::error::{AppError, AppResult};
use crate::protocol::messages::{Envelope, StudioInfo, PLUGIN_PROTOCOL_VERSION};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstallReport {
    pub(crate) built_plugin: String,
    pub(crate) installed_plugin: String,
    pub(crate) installed_hash: String,
    pub(crate) installed_modified_unix: u64,
    pub(crate) copied: bool,
    pub(crate) restart_studios: Vec<StudioRestart>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StudioRestart {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) protocol_version: Option<u32>,
    pub(crate) plugin_version: Option<String>,
    pub(crate) reason: String,
}

pub fn run(port: u16, watch: bool, json: bool) -> AppResult<()> {
    let mut last_stamp = None;
    loop {
        let report = install_once(port)?;
        if json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_report(&report);
        }
        if !watch {
            return Ok(());
        }
        println!("Watching plugin sources for changes. Press Ctrl+C to stop.");
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let stamp = latest_modified(&plugin_source_dir()?)?;
            if last_stamp.is_none() {
                last_stamp = Some(stamp);
            } else if Some(stamp) != last_stamp {
                last_stamp = Some(stamp);
                break;
            }
        }
    }
}

pub(crate) fn install_once(port: u16) -> AppResult<InstallReport> {
    let plugin_dir = plugin_dir()?;
    let built_plugin = plugin_dir.join("rs-bridge-plugin.rbxmx");
    build_plugin(&plugin_dir, &built_plugin)?;

    let installed_plugin = plugin_install_path();
    if let Some(parent) = installed_plugin.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let before_hash = file_hash(&installed_plugin).ok();
    std::fs::copy(&built_plugin, &installed_plugin)?;
    let after_hash = file_hash(&installed_plugin)?;
    let installed_modified = std::fs::metadata(&installed_plugin)?.modified()?;
    let restart_studios = restart_studios(port, before_hash.as_deref() != Some(&after_hash));

    Ok(InstallReport {
        built_plugin: built_plugin.display().to_string(),
        installed_plugin: installed_plugin.display().to_string(),
        installed_hash: after_hash,
        installed_modified_unix: unix_seconds(installed_modified),
        copied: true,
        restart_studios,
    })
}

fn build_plugin(plugin_dir: &Path, out: &Path) -> AppResult<()> {
    let output = Command::new("rojo")
        .arg("build")
        .arg("default.project.json")
        .arg("--output")
        .arg(out)
        .current_dir(plugin_dir)
        .output()
        .map_err(|err| AppError::Other(format!("could not run rojo build: {err}")))?;
    if !output.status.success() {
        return Err(AppError::Other(format!(
            "rojo build failed with {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

fn restart_studios(port: u16, copied_new_bundle: bool) -> Vec<StudioRestart> {
    let Ok(studios) = fetch_studios(port) else {
        return Vec::new();
    };
    studios
        .into_iter()
        .filter_map(|studio| {
            let reason = if copied_new_bundle {
                Some("plugin bundle was reinstalled".to_string())
            } else if studio.protocol_version != Some(PLUGIN_PROTOCOL_VERSION) {
                Some(format!(
                    "loaded protocol is {}, CLI expects {}",
                    studio
                        .protocol_version
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "unknown".into()),
                    PLUGIN_PROTOCOL_VERSION
                ))
            } else {
                None
            }?;
            Some(StudioRestart {
                id: studio.id,
                name: studio.name,
                protocol_version: studio.protocol_version,
                plugin_version: studio.plugin_version,
                reason,
            })
        })
        .collect()
}

fn fetch_studios(port: u16) -> AppResult<Vec<StudioInfo>> {
    let url = format!("http://127.0.0.1:{port}/studios");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    let env: Envelope<Vec<StudioInfo>> = client
        .get(&url)
        .send()
        .map_err(|source| crate::error::AppError::BridgeUnreachable {
            url: url.clone(),
            source,
        })?
        .json()?;
    if env.ok {
        Ok(env.data.unwrap_or_default())
    } else {
        Err(crate::cli::envelope_error(
            "install-plugin",
            env.error,
            env.code,
        ))
    }
}

fn print_report(report: &InstallReport) {
    println!("Built plugin: {}", report.built_plugin);
    println!("Installed plugin: {}", report.installed_plugin);
    println!("Installed hash: {}", report.installed_hash);
    if report.restart_studios.is_empty() {
        println!("No connected Studio restart required was detected.");
    } else {
        println!("Restart these Studio windows to load the installed plugin:");
        for studio in &report.restart_studios {
            println!("  - {} ({}) - {}", studio.name, studio.id, studio.reason);
        }
    }
}

fn plugin_dir() -> AppResult<PathBuf> {
    let current = std::env::current_dir()?;
    let direct = current.join("plugin");
    if direct.join("default.project.json").exists() {
        return Ok(direct);
    }
    if current.join("default.project.json").exists()
        && current.file_name().and_then(|v| v.to_str()) == Some("plugin")
    {
        return Ok(current);
    }
    Err(AppError::Other(
        "could not find plugin/default.project.json from current directory".into(),
    ))
}

fn plugin_source_dir() -> AppResult<PathBuf> {
    Ok(plugin_dir()?.join("src"))
}

fn plugin_install_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Roblox")
        .join("Plugins")
        .join("rs-bridge-plugin.rbxmx")
}

fn latest_modified(path: &Path) -> AppResult<SystemTime> {
    let mut latest = SystemTime::UNIX_EPOCH;
    fn walk(path: &Path, latest: &mut SystemTime) -> AppResult<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, latest)?;
            } else if let Ok(modified) = entry.metadata().and_then(|meta| meta.modified()) {
                if modified > *latest {
                    *latest = modified;
                }
            }
        }
        Ok(())
    }
    walk(path, &mut latest)?;
    Ok(latest)
}

fn file_hash(path: &Path) -> AppResult<String> {
    let bytes = std::fs::read(path)?;
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Ok(format!("{hash:016x}"))
}

fn unix_seconds(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
