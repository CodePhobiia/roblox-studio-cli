use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

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
    println!("API key stored locally and will not be printed by rs.");
    Ok(())
}

pub fn profile_list(json: bool) -> AppResult<()> {
    let config = load_config()?;
    if json {
        let redacted = config
            .profiles
            .iter()
            .map(|(name, profile)| {
                (
                    name.clone(),
                    serde_json::json!({
                        "creatorId": profile.creator_id,
                        "creatorType": profile.creator_type,
                        "hasApiKey": !profile.api_key.is_empty(),
                        "default": config.default_profile.as_deref() == Some(name)
                    }),
                )
            })
            .collect::<BTreeMap<_, _>>();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "path": config_path(),
                "defaultProfile": config.default_profile,
                "profiles": redacted
            }))?
        );
    } else if config.profiles.is_empty() {
        println!("No Open Cloud profiles configured.");
    } else {
        println!("Open Cloud profiles:");
        for (name, profile) in config.profiles {
            let marker = if Some(&name) == config.default_profile.as_ref() {
                "default"
            } else {
                ""
            };
            println!(
                "  - {name}: {} {} {}",
                profile.creator_type, profile.creator_id, marker
            );
        }
    }
    Ok(())
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
    std::fs::write(path, serde_json::to_string_pretty(config)?)?;
    Ok(())
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
    use super::validate_profile_name;

    #[test]
    fn profile_name_rejects_spaces() {
        assert!(validate_profile_name("mygroup").is_ok());
        assert!(validate_profile_name("my group").is_err());
    }
}
