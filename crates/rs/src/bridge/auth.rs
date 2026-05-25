use crate::error::AppResult;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const TOKEN_ENV: &str = "RS_BRIDGE_TOKEN";
pub const TOKEN_FILE_ENV: &str = "RS_BRIDGE_TOKEN_FILE";
pub const TOKEN_HEADER: &str = "x-rs-bridge-token";

pub fn load_or_create_token() -> AppResult<String> {
    if let Some(token) = env_token() {
        return Ok(token);
    }

    let path = token_path();
    match read_token_file(&path) {
        Ok(Some(token)) => Ok(token),
        Ok(None) => create_token_file(&path),
        Err(err) if err.kind() == ErrorKind::NotFound => create_token_file(&path),
        Err(err) => Err(err.into()),
    }
}

pub fn attach_blocking(
    builder: reqwest::blocking::RequestBuilder,
) -> AppResult<reqwest::blocking::RequestBuilder> {
    Ok(builder.header(TOKEN_HEADER, load_or_create_token()?))
}

fn env_token() -> Option<String> {
    std::env::var(TOKEN_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn token_path() -> PathBuf {
    if let Some(path) = std::env::var_os(TOKEN_FILE_ENV) {
        return PathBuf::from(path);
    }
    home_dir().join(".rs-bridge-token")
}

fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn read_token_file(path: &Path) -> std::io::Result<Option<String>> {
    let token = fs::read_to_string(path)?.trim().to_string();
    Ok((!token.is_empty()).then_some(token))
}

fn create_token_file(path: &Path) -> AppResult<String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let token = Uuid::new_v4().to_string();
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    match options.open(path) {
        Ok(mut file) => {
            writeln!(file, "{token}")?;
            Ok(token)
        }
        Err(err) if err.kind() == ErrorKind::AlreadyExists => read_token_file(path)?
            .ok_or_else(|| crate::error::AppError::Other("bridge token file is empty".into())),
        Err(err) => Err(err.into()),
    }
}
