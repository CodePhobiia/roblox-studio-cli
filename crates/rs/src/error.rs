use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("bridge unreachable at {url}: {source}")]
    BridgeUnreachable {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("couldn't start rs-bridge on port {port} within 3s. Check {log_path} for details.")]
    BridgeStartFailed { port: u16, log_path: String },

    #[error("studio '{name}' is not registered with the bridge. Open Studio and verify the plugin is installed.")]
    StudioNotConnected { name: String },

    #[error("ambiguous studio name '{name}'; candidates: {candidates:?}")]
    StudioAmbiguous {
        name: String,
        candidates: Vec<String>,
    },

    #[error("plugin error: {0}")]
    PluginError(String),

    #[error("command timed out after {timeout_ms}ms")]
    CommandTimeout { timeout_ms: u64 },

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::BridgeUnreachable { .. } | AppError::BridgeStartFailed { .. } => 2,
            AppError::StudioNotConnected { .. } | AppError::StudioAmbiguous { .. } => 3,
            AppError::PluginError(_) => 4,
            AppError::CommandTimeout { .. } => 5,
            _ => 1,
        }
    }

    pub fn bridge_code(&self) -> &'static str {
        match self {
            AppError::BridgeUnreachable { .. } | AppError::BridgeStartFailed { .. } => {
                "bridge-unreachable"
            }
            AppError::StudioNotConnected { .. } => "studio-not-found",
            AppError::StudioAmbiguous { .. } => "studio-ambiguous",
            AppError::PluginError(_) => "plugin-error",
            AppError::CommandTimeout { .. } => "command-timeout",
            AppError::Serde(_) => "bad-json",
            AppError::Io(_) => "io",
            AppError::Http(_) => "http",
            AppError::Other(_) => "internal",
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
