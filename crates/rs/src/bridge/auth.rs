use crate::error::AppResult;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const TOKEN_ENV: &str = "RS_BRIDGE_TOKEN";
pub const TOKEN_FILE_ENV: &str = "RS_BRIDGE_TOKEN_FILE";
pub const TOKEN_HEADER: &str = "x-rs-bridge-token";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenInfo {
    pub token: String,
    pub source: TokenSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSource {
    Env { var: &'static str },
    File { path: PathBuf, created: bool },
}

pub fn load_or_create_token() -> AppResult<String> {
    Ok(load_or_create_token_info()?.token)
}

pub fn load_or_create_token_info() -> AppResult<TokenInfo> {
    if let Some(token) = env_token() {
        return Ok(TokenInfo {
            token,
            source: TokenSource::Env { var: TOKEN_ENV },
        });
    }

    let path = token_path();
    match read_token_file(&path) {
        Ok(Some(token)) => Ok(TokenInfo {
            token,
            source: TokenSource::File {
                path,
                created: false,
            },
        }),
        Ok(None) => Ok(TokenInfo {
            token: create_token_file(&path)?,
            source: TokenSource::File {
                path,
                created: true,
            },
        }),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(TokenInfo {
            token: create_token_file(&path)?,
            source: TokenSource::File {
                path,
                created: true,
            },
        }),
        Err(err) => Err(err.into()),
    }
}

pub fn attach_blocking(
    builder: reqwest::blocking::RequestBuilder,
) -> AppResult<reqwest::blocking::RequestBuilder> {
    Ok(builder.header(TOKEN_HEADER, load_or_create_token()?))
}

pub fn unauthorized_message(has_header: bool, expected_source: &TokenSource) -> String {
    let header_state = if has_header { "invalid" } else { "missing" };
    format!(
        "bridge request has {header_state} {TOKEN_HEADER}; the running bridge is using {}. \
This usually means a stale CLI token, stale bridge token, missing header, or different \
{TOKEN_FILE_ENV}. Non-destructive recovery: verify {TOKEN_ENV} in this terminal, verify \
{TOKEN_FILE_ENV} points at the same token file, or use the matching token source before retrying. \
To keep current Studio sessions intact, start another bridge on a different --port if you need a \
different token.",
        expected_source.describe()
    )
}

impl TokenSource {
    fn describe(&self) -> String {
        match self {
            TokenSource::Env { var } => format!("{var} from the bridge process environment"),
            TokenSource::File { path, created } => {
                let freshness = if *created { "newly created " } else { "" };
                format!("{freshness}token file {}", path.display())
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    struct EnvSnapshot {
        token: Option<std::ffi::OsString>,
        token_file: Option<std::ffi::OsString>,
    }

    impl EnvSnapshot {
        fn capture() -> Self {
            Self {
                token: std::env::var_os(TOKEN_ENV),
                token_file: std::env::var_os(TOKEN_FILE_ENV),
            }
        }
    }

    impl Drop for EnvSnapshot {
        fn drop(&mut self) {
            match &self.token {
                Some(value) => std::env::set_var(TOKEN_ENV, value),
                None => std::env::remove_var(TOKEN_ENV),
            }
            match &self.token_file {
                Some(value) => std::env::set_var(TOKEN_FILE_ENV, value),
                None => std::env::remove_var(TOKEN_FILE_ENV),
            }
        }
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn temp_token_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rs-bridge-{name}-{}.token", Uuid::new_v4()))
    }

    #[test]
    fn load_or_create_token_prefers_env_over_file() {
        let _guard = env_lock();
        let _env = EnvSnapshot::capture();
        let path = temp_token_path("env-precedence");
        std::fs::write(&path, "file-token\n").unwrap();
        std::env::set_var(TOKEN_FILE_ENV, &path);
        std::env::set_var(TOKEN_ENV, " env-token ");

        let info = load_or_create_token_info().unwrap();

        assert_eq!(info.token, "env-token");
        assert_eq!(info.source, TokenSource::Env { var: TOKEN_ENV });

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_or_create_token_reads_existing_token_file_from_env_path() {
        let _guard = env_lock();
        let _env = EnvSnapshot::capture();
        let path = temp_token_path("existing-file");
        std::fs::write(&path, " file-token \n").unwrap();
        std::env::remove_var(TOKEN_ENV);
        std::env::set_var(TOKEN_FILE_ENV, &path);

        let info = load_or_create_token_info().unwrap();

        assert_eq!(info.token, "file-token");
        assert_eq!(
            info.source,
            TokenSource::File {
                path: path.clone(),
                created: false
            }
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_or_create_token_creates_missing_token_file_from_env_path() {
        let _guard = env_lock();
        let _env = EnvSnapshot::capture();
        let path = temp_token_path("created-file");
        std::env::remove_var(TOKEN_ENV);
        std::env::set_var(TOKEN_FILE_ENV, &path);

        let info = load_or_create_token_info().unwrap();

        assert!(!info.token.is_empty());
        assert_eq!(
            info.source,
            TokenSource::File {
                path: path.clone(),
                created: true
            }
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap().trim(), info.token);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unauthorized_message_names_stale_token_recovery_without_shutdown() {
        let message = unauthorized_message(
            false,
            &TokenSource::File {
                path: PathBuf::from("C:\\tmp\\rs.token"),
                created: false,
            },
        );

        assert!(message.contains("missing x-rs-bridge-token"));
        assert!(message.contains("stale CLI token"));
        assert!(message.contains(TOKEN_ENV));
        assert!(message.contains(TOKEN_FILE_ENV));
        assert!(message.contains("different --port"));
    }
}
