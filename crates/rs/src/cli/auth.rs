use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(unix)]
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCloudProfile {
    pub creator_id: u64,
    pub creator_type: String,
    pub api_key: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    #[serde(default)]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, OpenCloudProfile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RedactedOpenCloudProfile {
    creator_id: u64,
    creator_type: String,
    has_api_key: bool,
    default: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthProfileDoctorReport {
    ok: bool,
    path: PathBuf,
    profile_count: usize,
    api_key_profile_count: usize,
    storage: StorageSecurityCheck,
    warnings: Vec<String>,
    fixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageSecurityCheck {
    status: String,
    message: String,
    plaintext_api_keys: bool,
    permission_detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PermissionAssessment {
    Missing,
    #[cfg(any(unix, test))]
    Restricted(String),
    #[cfg(any(unix, test))]
    Insecure(String),
    Unknown(String),
}

pub fn profile_add(
    name: String,
    creator_id: u64,
    creator_type: String,
    api_key: Option<String>,
    set_default: bool,
) -> AppResult<()> {
    validate_profile_name(&name)?;
    let api_key = api_key
        .or_else(|| std::env::var("ROBLOX_API_KEY").ok())
        .ok_or_else(|| {
            AppError::Other("missing API key; pass --api-key or set ROBLOX_API_KEY".into())
        })?;
    let mut config = load_config()?;
    config.profiles.insert(
        name.clone(),
        OpenCloudProfile {
            creator_id,
            creator_type,
            api_key,
        },
    );
    if set_default || config.default_profile.is_none() {
        config.default_profile = Some(name.clone());
    }
    save_config(&config)?;
    println!(
        "Saved Open Cloud profile '{name}' to {}",
        config_path().display()
    );
    println!(
        "WARNING: API key is stored locally in plaintext; rs restricts file permissions where supported and never prints saved keys."
    );
    Ok(())
}

pub fn profile_list(json: bool) -> AppResult<()> {
    let config = load_config()?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&profile_list_json_value(&config, config_path()))?
        );
    } else {
        for line in profile_list_text_lines(&config) {
            println!("{line}");
        }
    }
    Ok(())
}

pub fn profile_doctor(json: bool) -> AppResult<()> {
    let path = config_path();
    let config = load_config()?;
    let permissions = inspect_config_file_permissions(&path)?;
    let report = build_profile_doctor_report(&config, path, permissions);
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_profile_doctor_report(&report);
    }
    if report.storage.status == "fail" {
        Err(AppError::Other(
            "auth profile doctor found insecure credential storage".into(),
        ))
    } else {
        Ok(())
    }
}

pub fn profile_remove(name: String) -> AppResult<()> {
    let mut config = load_config()?;
    if config.profiles.remove(&name).is_none() {
        return Err(AppError::Other(format!("profile '{name}' does not exist")));
    }
    if config.default_profile.as_deref() == Some(&name) {
        config.default_profile = config.profiles.keys().next().cloned();
    }
    save_config(&config)?;
    println!("Removed Open Cloud profile '{name}'.");
    Ok(())
}

pub fn profile_default(name: String) -> AppResult<()> {
    let mut config = load_config()?;
    if !config.profiles.contains_key(&name) {
        return Err(AppError::Other(format!("profile '{name}' does not exist")));
    }
    config.default_profile = Some(name.clone());
    save_config(&config)?;
    println!("Default Open Cloud profile is now '{name}'.");
    Ok(())
}

pub fn resolve_profile(name: Option<&str>) -> AppResult<Option<OpenCloudProfile>> {
    let Some(name) = name else {
        return Ok(None);
    };
    let config = load_config()?;
    let profile_name = if name == "default" {
        config
            .default_profile
            .as_deref()
            .ok_or_else(|| AppError::Other("no default Open Cloud profile configured".into()))?
            .to_string()
    } else {
        name.to_string()
    };
    config
        .profiles
        .get(&profile_name)
        .cloned()
        .map(Some)
        .ok_or_else(|| {
            AppError::Other(format!(
                "Open Cloud profile '{profile_name}' does not exist"
            ))
        })
}

fn validate_profile_name(name: &str) -> AppResult<()> {
    if name.trim().is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
        || name.contains(char::is_whitespace)
    {
        return Err(AppError::Other(
            "profile name must be non-empty and contain no spaces, slashes, or ':'".into(),
        ));
    }
    Ok(())
}

fn load_config() -> AppResult<AuthConfig> {
    let path = config_path();
    if !path.exists() {
        return Ok(AuthConfig::default());
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn save_config(config: &AuthConfig) -> AppResult<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_config_file(&path, serde_json::to_string_pretty(config)?.as_bytes())?;
    harden_config_permissions(&path)?;
    Ok(())
}

#[cfg(unix)]
fn write_config_file(path: &Path, contents: &[u8]) -> AppResult<()> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_config_file(path: &Path, contents: &[u8]) -> AppResult<()> {
    std::fs::write(path, contents)?;
    Ok(())
}

#[cfg(unix)]
fn harden_config_permissions(path: &Path) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;

    if let Some(parent) = path.parent() {
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn harden_config_permissions(_path: &Path) -> AppResult<()> {
    Ok(())
}

fn profile_list_json_value(config: &AuthConfig, path: PathBuf) -> serde_json::Value {
    serde_json::json!({
        "path": path,
        "defaultProfile": config.default_profile.as_deref(),
        "profiles": redacted_profiles(config),
    })
}

fn redacted_profiles(config: &AuthConfig) -> BTreeMap<String, RedactedOpenCloudProfile> {
    config
        .profiles
        .iter()
        .map(|(name, profile)| {
            (
                name.clone(),
                RedactedOpenCloudProfile {
                    creator_id: profile.creator_id,
                    creator_type: profile.creator_type.clone(),
                    has_api_key: !profile.api_key.is_empty(),
                    default: config.default_profile.as_deref() == Some(name),
                },
            )
        })
        .collect()
}

fn profile_list_text_lines(config: &AuthConfig) -> Vec<String> {
    if config.profiles.is_empty() {
        return vec!["No Open Cloud profiles configured.".into()];
    }

    let mut lines = vec!["Open Cloud profiles:".into()];
    for (name, profile) in &config.profiles {
        let marker = if config.default_profile.as_deref() == Some(name) {
            "default"
        } else {
            ""
        };
        lines.push(format!(
            "  - {name}: {} {} {}",
            profile.creator_type, profile.creator_id, marker
        ));
    }
    lines
}

fn build_profile_doctor_report(
    config: &AuthConfig,
    path: PathBuf,
    permissions: PermissionAssessment,
) -> AuthProfileDoctorReport {
    let profile_count = config.profiles.len();
    let api_key_profile_count = config
        .profiles
        .values()
        .filter(|profile| !profile.api_key.is_empty())
        .count();
    let plaintext_api_keys = api_key_profile_count > 0;
    let storage = storage_security_check(plaintext_api_keys, permissions);
    let mut warnings = Vec::new();
    let mut fixes = Vec::new();

    if plaintext_api_keys {
        warnings.push(format!(
            "{api_key_profile_count} Open Cloud profile(s) contain locally stored plaintext API keys."
        ));
        fixes.push(
            "Prefer ROBLOX_API_KEY for short-lived sessions or remove profiles when they are no longer needed.".into(),
        );
    }
    match storage.status.as_str() {
        "fail" => fixes.push(format!(
            "Restrict {} so only the current OS user can read and write it.",
            path.display()
        )),
        "warn" if storage.permission_detail.is_some() => warnings.push(storage.message.clone()),
        _ => {}
    }

    AuthProfileDoctorReport {
        ok: storage.status != "fail",
        path,
        profile_count,
        api_key_profile_count,
        storage,
        warnings,
        fixes,
    }
}

fn storage_security_check(
    plaintext_api_keys: bool,
    permissions: PermissionAssessment,
) -> StorageSecurityCheck {
    if !plaintext_api_keys {
        return StorageSecurityCheck {
            status: "pass".into(),
            message: "no saved Open Cloud API keys were found".into(),
            plaintext_api_keys,
            permission_detail: None,
        };
    }

    match permissions {
        PermissionAssessment::Missing => StorageSecurityCheck {
            status: "warn".into(),
            message: "profile config contains API keys in memory, but the config file is missing"
                .into(),
            plaintext_api_keys,
            permission_detail: None,
        },
        #[cfg(any(unix, test))]
        PermissionAssessment::Restricted(detail) => StorageSecurityCheck {
            status: "warn".into(),
            message: "Open Cloud API keys are stored in local plaintext; file permissions are restricted where rs can verify them".into(),
            plaintext_api_keys,
            permission_detail: Some(detail),
        },
        #[cfg(any(unix, test))]
        PermissionAssessment::Insecure(detail) => StorageSecurityCheck {
            status: "fail".into(),
            message: "Open Cloud API keys are stored in local plaintext and profile file permissions are too broad".into(),
            plaintext_api_keys,
            permission_detail: Some(detail),
        },
        PermissionAssessment::Unknown(detail) => StorageSecurityCheck {
            status: "warn".into(),
            message: "Open Cloud API keys are stored in local plaintext and rs cannot verify OS ACLs on this platform without a credential-store integration".into(),
            plaintext_api_keys,
            permission_detail: Some(detail),
        },
    }
}

fn inspect_config_file_permissions(path: &Path) -> AppResult<PermissionAssessment> {
    if !path.exists() {
        return Ok(PermissionAssessment::Missing);
    }
    platform_permission_assessment(path)
}

#[cfg(unix)]
fn platform_permission_assessment(path: &Path) -> AppResult<PermissionAssessment> {
    use std::os::unix::fs::PermissionsExt;

    let mode = std::fs::metadata(path)?.permissions().mode() & 0o777;
    let detail = format!("mode {mode:03o}");
    if mode & 0o077 == 0 {
        Ok(PermissionAssessment::Restricted(detail))
    } else {
        Ok(PermissionAssessment::Insecure(detail))
    }
}

#[cfg(not(unix))]
fn platform_permission_assessment(_path: &Path) -> AppResult<PermissionAssessment> {
    Ok(PermissionAssessment::Unknown(
        "ACL verification is not implemented in this alpha build".into(),
    ))
}

fn print_profile_doctor_report(report: &AuthProfileDoctorReport) {
    println!("rs auth profile doctor");
    println!("Path: {}", report.path.display());
    println!("Profiles: {}", report.profile_count);
    println!(
        "Profiles with saved API keys: {}",
        report.api_key_profile_count
    );
    println!(
        "{}  Storage: {}",
        report.storage.status.to_ascii_uppercase(),
        report.storage.message
    );
    if let Some(detail) = &report.storage.permission_detail {
        println!("INFO  Permissions: {detail}");
    }
    for warning in &report.warnings {
        println!("WARN  {warning}");
    }
    for fix in &report.fixes {
        println!("FIX   {fix}");
    }
}

fn config_path() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("rs").join("profiles.json");
    }
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        return PathBuf::from(home).join(".rs").join("profiles.json");
    }
    PathBuf::from(".rs").join("profiles.json")
}

#[cfg(test)]
mod tests {
    use super::{
        build_profile_doctor_report, profile_list_json_value, profile_list_text_lines,
        validate_profile_name, AuthConfig, OpenCloudProfile, PermissionAssessment,
    };
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn profile_name_rejects_spaces() {
        assert!(validate_profile_name("mygroup").is_ok());
        assert!(validate_profile_name("my group").is_err());
    }

    #[test]
    fn profile_list_json_redacts_api_keys() {
        let config = config_with_secret("secret-profile-key");

        let value = profile_list_json_value(&config, PathBuf::from("profiles.json"));
        let text = serde_json::to_string(&value).unwrap();

        assert!(!text.contains("secret-profile-key"));
        assert_eq!(value["profiles"]["alpha"]["hasApiKey"], true);
        assert!(value["profiles"]["alpha"].get("apiKey").is_none());
    }

    #[test]
    fn profile_list_text_redacts_api_keys() {
        let config = config_with_secret("secret-profile-key");

        let text = profile_list_text_lines(&config).join("\n");

        assert!(!text.contains("secret-profile-key"));
        assert!(text.contains("alpha"));
        assert!(text.contains("default"));
    }

    #[test]
    fn profile_doctor_detects_plaintext_storage_when_permissions_unknown() {
        let config = config_with_secret("secret-profile-key");

        let report = build_profile_doctor_report(
            &config,
            PathBuf::from("profiles.json"),
            PermissionAssessment::Unknown("test platform".into()),
        );

        assert!(report.ok);
        assert_eq!(report.storage.status, "warn");
        assert!(report.storage.plaintext_api_keys);
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("plaintext API keys")));
    }

    #[test]
    fn profile_doctor_flags_broad_profile_file_permissions() {
        let config = config_with_secret("secret-profile-key");

        let report = build_profile_doctor_report(
            &config,
            PathBuf::from("profiles.json"),
            PermissionAssessment::Insecure("mode 0644".into()),
        );

        assert!(!report.ok);
        assert_eq!(report.storage.status, "fail");
        assert_eq!(
            report.storage.permission_detail.as_deref(),
            Some("mode 0644")
        );
        assert!(report
            .fixes
            .iter()
            .any(|fix| fix.contains("only the current OS user")));
    }

    #[test]
    fn profile_doctor_warns_when_plaintext_file_permissions_are_restricted() {
        let config = config_with_secret("secret-profile-key");

        let report = build_profile_doctor_report(
            &config,
            PathBuf::from("profiles.json"),
            PermissionAssessment::Restricted("mode 0600".into()),
        );

        assert!(report.ok);
        assert_eq!(report.storage.status, "warn");
        assert_eq!(
            report.storage.permission_detail.as_deref(),
            Some("mode 0600")
        );
    }

    fn config_with_secret(secret: &str) -> AuthConfig {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "alpha".into(),
            OpenCloudProfile {
                creator_id: 123,
                creator_type: "group".into(),
                api_key: secret.into(),
            },
        );
        AuthConfig {
            default_profile: Some("alpha".into()),
            profiles,
        }
    }
}
